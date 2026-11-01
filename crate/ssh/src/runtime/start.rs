//! The runtime's start and shutdown, one runtime a process.
use crate::*;

/// Whether this process has started its runtime, which it does at most once.
static STARTED: AtomicBool = AtomicBool::new(false);

/// What a worker's lock record names its holder.
const LOCK_HOLDER: &str = "grammar-ssh";

/// How long a shutdown waits for the worker's tasks to end.
const SHUTDOWN_GRACE: std::time::Duration = std::time::Duration::from_secs(2);

/// A worker's immutable identity: its effective UID, MCP id, and namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeIdentity {
    /// The effective Unix UID the worker runs as.
    pub uid: u32,
    /// The MCP id owning the state namespace.
    pub id: String,
    /// The state namespace within the id.
    pub namespace: String,
}

/// The worker's runtime, which its host starts and shuts down.
#[derive(Debug)]
pub struct Runtime {
    config: InstanceConfig,
    nom: String,
    scratch: WorkerScratch,
    diagnostics: Vec<String>,
    tasks: tk::runtime::Runtime,
    controller: GrammarSshController,
    lock: NamespaceLock,
}

impl Runtime {
    /// Starts the process's one runtime on `config`: its lock, its scratch, its engine.
    pub fn start(config: InstanceConfig) -> GrammarSshResult<Self> {
        if STARTED.swap(true, Ordering::AcqRel) {
            return Err(GrammarSshError::AlreadyStarted);
        }
        let given_back = |e: GrammarSshError| {
            STARTED.store(false, Ordering::Release);
            e
        };
        let nom = datum::Nom::generate(&datum::NonceGenerator::new()).into_pair();
        let (lock, scratch) = claim(&config, nom.as_str()).map_err(given_back)?;
        let tasks = match tasks() {
            Ok(tasks) => tasks,
            Err(e) => {
                let _ = scratch.remove();
                return Err(given_back(e));
            }
        };
        let mut diagnostics = scratch.diagnostics().to_vec();
        let started = start_engine(&config, &scratch, nom.as_str(), &mut diagnostics)
            .and_then(|()| tasks.block_on(GrammarSshController::start(&config)));
        let controller = match started {
            Ok(controller) => controller,
            Err(e) => {
                tasks.shutdown_timeout(SHUTDOWN_GRACE);
                let _ = scratch.remove();
                return Err(e);
            }
        };
        Ok(Self {
            config,
            nom: nom.as_str().to_string(),
            scratch,
            diagnostics,
            tasks,
            controller,
            lock,
        })
    }

    /// The identity the runtime started under.
    pub fn identity(&self) -> &RuntimeIdentity {
        &self.config.identity
    }

    /// The worker's nom, drawn at its start, its engine's `$env.EQUIP_MCP_NOM`.
    pub fn nom(&self) -> &str {
        &self.nom
    }

    /// The worker's scratch directories.
    pub fn scratch(&self) -> &WorkerScratch {
        &self.scratch
    }

    /// What the start found to report: the scratch's and the substrate's.
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    /// The worker's identity and bounds, as its reply to a hello names them.
    pub fn profile(&self) -> GrammarSshResult<WorkerProfile> {
        let instance = Instance::new(&self.config.identity.id, &self.config.identity.namespace)
            .map_err(|e| GrammarSshError::Session { reason: e.message() })?;
        Ok(WorkerProfile {
            instance,
            uid: self.config.identity.uid,
            pid: process::id(),
            nom: self.nom.clone(),
            limits: LIMITS_DEFAULT,
        })
    }

    /// The worker's identity, homes, and plugins, as `interface info` reports them.
    pub fn info(&self) -> WorkerInfo {
        WorkerInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            nu_version: engine::NU_VERSION.to_string(),
            id: self.config.identity.id.clone(),
            namespace: self.config.identity.namespace.clone(),
            nom: self.nom.clone(),
            work_dir: self.config.workdir.display().to_string(),
            tmp: self.scratch.tmp().display().to_string(),
            transient: self.scratch.transient().display().to_string(),
            plugins: engine::list_registered_plugins(),
        }
    }

    /// A session served on `reader` and `writer` until it ends, on the worker's tasks.
    pub fn serve<R, W>(
        &self,
        reader: R,
        writer: W,
    ) -> GrammarSshResult<SessionEnd>
    where
        R: tk::AsyncRead + Unpin + Send + 'static,
        W: tk::AsyncWrite + Unpin + Send + 'static,
    {
        let profile = self.profile()?;
        let info = self.info();
        self.tasks.block_on(serve(reader, writer, profile, info, &self.controller))
    }

    /// Shuts the runtime down: its systems, its tasks, its scratch, its lock last.
    pub fn shutdown(self) -> GrammarSshResult<()> {
        let Self {
            scratch,
            tasks,
            controller,
            lock,
            ..
        } = self;
        tasks.block_on(controller.shutdown());
        tasks.shutdown_timeout(SHUTDOWN_GRACE);
        let removed = scratch.remove();
        drop(lock);
        removed
    }
}

/// The worker's task runtime, its threads sized for the parses run on them.
fn tasks() -> GrammarSshResult<tk::runtime::Runtime> {
    tk::runtime::Builder::new_multi_thread()
        .thread_name("grammar-ssh")
        .thread_stack_size(engine::EVAL_STACK_SIZE)
        .enable_all()
        .build()
        .map_err(|e| GrammarSshError::TasksUnavailable { reason: e.to_string() })
}

/// The lock taken and its record written, then the scratch made.
fn claim(
    config: &InstanceConfig,
    nom: &str,
) -> GrammarSshResult<(NamespaceLock, WorkerScratch)> {
    let unavailable = |reason: String| GrammarSshError::LockUnavailable {
        path: config.paths.lock.clone(),
        reason,
    };
    let mut lock = NamespaceLock::acquire(&config.paths.lock)?;
    let record = LockRecord::current(LOCK_HOLDER, &config.identity.id, &config.identity.namespace, nom)
        .map_err(|e| unavailable(e.to_string()))?;
    lock.record(&record)?;
    let scratch = WorkerScratch::create(&config.homes)?;
    Ok((lock, scratch))
}

/// The engine started once on the instance's homes and scratch, its substrate made.
fn start_engine(
    config: &InstanceConfig,
    scratch: &WorkerScratch,
    nom: &str,
    diagnostics: &mut Vec<String>,
) -> GrammarSshResult<()> {
    let failed = |reason: String| GrammarSshError::EngineInit { reason };
    engine::init(engine::EngineParams {
        id: config.identity.id.clone(),
        namespace: config.identity.namespace.clone(),
        work_dir: config.workdir.clone(),
        tmp_dir: scratch.tmp().to_path_buf(),
        transient_dir: scratch.transient().to_path_buf(),
        mcp_nom: nom.to_string(),
        data_home: config.paths.data.clone(),
        cache_home: config.paths.cache.clone(),
    })
    .map_err(|e| failed(e.to_string()))?;
    let fresh = engine::ensure_equip_store().map_err(|e| failed(format!("the equip store: {e}")))?;
    if let Err(e) = engine::ensure_default_purview() {
        diagnostics.push(format!("the default purview is not made: {e:?}"));
    }
    engine::ensure_plugin_registry();
    if fresh {
        engine::seed_builtin_purviews(&engine::install_builtins());
    }
    Ok(())
}
