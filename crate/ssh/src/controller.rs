//! grammar-ssh's controller: its one engine, its operator, and its lifetime.
use crate::*;

/// How long the shutdown waits for in-flight requests to let the engine go.
const ENGINE_RELEASE_GRACE: std::time::Duration = std::time::Duration::from_secs(1);

/// What the controller's record in its namespace lock names its holder.
const LOCK_HOLDER: &str = "grammar-ssh";

/// A system's status change, logged unless a shutdown is running.
pub(crate) fn log_status(
    system: &str,
    sub: subsys::FromSub,
    stopping: &AtomicBool,
) {
    if stopping.load(Ordering::SeqCst) {
        return;
    }
    if let subsys::FromSub::StatusChange(change) = sub {
        eprintln!("grammar: {system}: {:?}", change.msg().0);
    }
}

/// A system's end its control did not ask for, logged unless a shutdown is running.
pub(crate) fn log_closed(
    system: &str,
    stopping: &AtomicBool,
) {
    if !stopping.load(Ordering::SeqCst) {
        eprintln!("grammar: {system} ended unasked");
    }
}

/// A slot's lock, a poisoned one taken as it stands.
pub(crate) fn held<T>(slot: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    slot.lock().unwrap_or_else(|e| e.into_inner())
}

/// `lock`'s record written: this process its holder, under its engine's `nom`.
fn record_lock(
    lock: &mut NamespaceLock,
    config: &Config,
    nom: &str,
) -> GrammarSshResult<()> {
    let record = LockRecord::current(LOCK_HOLDER, &config.id, &config.namespace, nom).map_err(|e| {
        GrammarSshError::LockUnavailable {
            path: config.lock_file(),
            reason: e.to_string(),
        }
    })?;
    Ok(lock.record(&record)?)
}

/// The namespace's cache home removed whole, its logs among it, never past a link.
fn remove_cache_home(path: &Path) -> GrammarSshResult<()> {
    let unavailable = |reason: String| GrammarSshError::CacheUnavailable {
        path: path.to_path_buf(),
        reason,
    };
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(unavailable(e.to_string())),
        Ok(meta) if !meta.is_dir() => Err(unavailable("it is not a directory".to_string())),
        Ok(_) => fs::remove_dir_all(path).map_err(|e| unavailable(e.to_string())),
    }
}

/// The operator and the observer that feeds it.
struct Supervision {
    operator: Arc<OperatorHandle>,
    observer: tk::JoinHandle<()>,
}

/// grammar-ssh's systems, its namespace's lock and its scratch held for their life.
pub(crate) struct GrammarSshController {
    /// The config the process resolved, which every system starts on.
    config: Arc<Config>,
    /// The one engine's system, none once the shutdown takes it.
    engine: std::sync::Mutex<Option<Arc<EngineHandle>>>,
    /// The engine's identity, which grammar-ssh inherits.
    identity: EngineIdentity,
    /// The operator and its observer.
    supervision: std::sync::Mutex<Option<Supervision>>,
    /// The session's MCP, for the engine's asks and the policy's emergencies.
    asker: Arc<Asker>,
    /// The scratch directories, removed at the end.
    scratch: ScratchDirs,
    /// The namespace's lock, released last.
    lock: std::sync::Mutex<Option<NamespaceLock>>,
    /// Set as the shutdown starts, so the systems' ends it asks for go unsaid.
    stopping: Arc<AtomicBool>,
}

impl fmt::Debug for GrammarSshController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GrammarSshController").finish_non_exhaustive()
    }
}

impl GrammarSshController {
    /// The one engine started, its nom the lock's and the log's, then the operator.
    pub(crate) async fn start(
        config: Arc<Config>,
        scratch: ScratchDirs,
        lock: NamespaceLock,
    ) -> GrammarSshResult<Self> {
        let asker = Arc::new(Asker::default());
        let stopping = Arc::new(AtomicBool::new(false));
        let route = HostRoute {
            asker: asker.clone(),
            supervisor: config.supervisor,
            stopping: stopping.clone(),
        };
        let engine = match EngineHandle::start(route).await {
            Ok(engine) => Arc::new(engine),
            Err(e) => {
                let _ = scratch.remove();
                return Err(GrammarSshError::EngineInit { reason: e.to_string() });
            }
        };
        let controller = Self {
            identity: engine.identity().clone(),
            config,
            engine: std::sync::Mutex::new(Some(engine)),
            supervision: std::sync::Mutex::new(None),
            asker,
            scratch,
            lock: std::sync::Mutex::new(Some(lock)),
            stopping,
        };
        if let Err(e) = controller.supervise().await {
            let _ = controller.shutdown().await;
            return Err(e);
        }
        Ok(controller)
    }

    /// The lock's record under the engine's nom, then the operator, observed.
    async fn supervise(&self) -> GrammarSshResult<()> {
        let failed = |reason: String| GrammarSshError::EngineInit { reason };
        let nom = self.nom();
        if let Some(lock) = held(&self.lock).as_mut() {
            record_lock(lock, &self.config, nom)?;
        }
        let log = emergency_log_path(&self.config.cache_home(), nom);
        let policy = EmergencyPolicy::new(self.asker.clone(), nom, log);
        let paths = OperatorSysPaths::new(self.config.data_home(), self.config.cache_home());
        let params = OperatorSysParams {
            test: false,
            supervisor: self.config.supervisor,
        };
        let watch = OperatorWatch {
            policy,
            stopping: self.stopping.clone(),
        };
        let operator = Arc::new(OperatorHandle::start(paths, params, watch).await.map_err(failed)?);
        let engine = self.engine().map_err(|e| failed(e.to_string()))?;
        let observer = spawn_observer(engine, operator.clone(), self.asker.clone());
        *held(&self.supervision) = Some(Supervision { operator, observer });
        Ok(())
    }

    /// The config the process resolved.
    pub(crate) fn config(&self) -> &Config {
        &self.config
    }

    /// grammar-ssh's nom, its engine's.
    pub(crate) fn nom(&self) -> &str {
        self.identity.nom().as_str()
    }

    /// The engine's system, or the failure a stopped one answers.
    pub(crate) fn engine(&self) -> EngineResult<Arc<EngineHandle>> {
        held(&self.engine).clone().ok_or_else(|| EngineError::Internal {
            phase: "ssh::engine".to_string(),
            reason: "the engine's system has stopped".to_string(),
        })
    }

    /// The way to the session's MCP.
    pub(crate) fn asker(&self) -> &Arc<Asker> {
        &self.asker
    }

    /// What grammar-ssh's hello answers from: its namespace, its process, its nom.
    pub(crate) fn profile(&self) -> GrammarSshResult<SshProfile> {
        let instance = Instance::new(&self.config.id, &self.config.namespace)
            .map_err(|e| GrammarSshError::Session { reason: e.message() })?;
        Ok(SshProfile {
            instance,
            uid: self.config.uid,
            pid: process::id(),
            nom: self.nom().to_string(),
            limits: LIMITS_DEFAULT,
        })
    }

    /// grammar-ssh's identity, homes, and plugins, as `interface info` reports them.
    pub(crate) fn info(&self) -> SshInfo {
        SshInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            nu_version: engine::NU_VERSION.to_string(),
            id: self.config.id.clone(),
            namespace: self.config.namespace.clone(),
            nom: self.nom().to_string(),
            work_dir: self.config.workdir.display().to_string(),
            tmp: self.scratch.tmp().display().to_string(),
            transient: self.scratch.transient().display().to_string(),
            plugins: engine::list_registered_plugins(),
        }
    }

    /// The session served on `reader` and `writer` until it ends.
    pub(crate) async fn serve<R, W>(
        &self,
        reader: R,
        writer: W,
    ) -> GrammarSshResult<SessionEnd>
    where
        R: tk::AsyncRead + Unpin + Send + 'static,
        W: tk::AsyncWrite + Unpin + Send + 'static,
    {
        serve(reader, writer, self.profile()?, self.info(), self).await
    }

    /// Every call in flight cancelled and its process tree reaped.
    pub(crate) async fn teardown(&self) {
        if let Ok(engine) = self.engine()
            && let Err(e) = engine.control(EngineCommand::Teardown).await
        {
            eprintln!("grammar: the engine's teardown: {e}");
        }
    }

    /// The end: systems stopped, scratch and cache home removed, the lock last.
    pub(crate) async fn shutdown(&self) -> GrammarSshResult<()> {
        self.stopping.store(true, Ordering::SeqCst);
        let supervision = held(&self.supervision).take();
        let operator = match supervision {
            Some(Supervision { operator, observer }) => {
                observer.abort();
                let _ = observer.await;
                Some(operator)
            }
            None => None,
        };
        self.teardown().await;
        if let Some(operator) = operator
            && let Ok(operator) = Arc::try_unwrap(operator)
            && let Err(e) = operator.stop().await
        {
            eprintln!("grammar: {e}");
        }
        self.stop_engine().await;
        let scratch = self.scratch.remove();
        let cache = remove_cache_home(&self.config.cache_home());
        drop(held(&self.lock).take());
        scratch.and(cache)
    }

    /// The engine stopped once in-flight requests let it go, or left within the grace.
    async fn stop_engine(&self) {
        let Some(mut engine) = held(&self.engine).take() else {
            return;
        };
        let deadline = std::time::Instant::now() + ENGINE_RELEASE_GRACE;
        loop {
            match Arc::try_unwrap(engine) {
                Ok(handle) => {
                    if let Err(e) = handle.stop().await {
                        eprintln!("grammar: the engine's stop: {e}");
                    }
                    return;
                }
                Err(shared) if std::time::Instant::now() < deadline => {
                    engine = shared;
                    tk::sleep(std::time::Duration::from_millis(10)).await;
                }
                Err(_) => return,
            }
        }
    }
}
