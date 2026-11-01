//! In-process nushell evaluation: the host-side eval engine.
use crate::*;

/// Flips a liveness flag on drop, so a hung thread is distinguishable.
struct FinishGuard(Arc<AtomicBool>);

impl Drop for FinishGuard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// `EngineState::merge_env` minus the process chdir.
pub(crate) fn merge_env_no_chdir(
    engine_state: &mut nu::EngineState,
    stack: &mut nu::Stack,
) {
    for mut scope in stack.env_vars.drain(..) {
        for (overlay_name, mut env) in Arc::make_mut(&mut scope).drain() {
            if let Some(env_vars) =
                Arc::make_mut(&mut engine_state.env_vars).get_mut(&overlay_name)
            {
                env_vars.extend(env.drain());
            } else {
                Arc::make_mut(&mut engine_state.env_vars).insert(overlay_name, env);
            }
        }
    }
    if let Some(config) = stack.config.take() {
        engine_state.set_config(config);
    }
}

/// A call's log, created empty, then opened to append so every writer's bytes land.
fn open_log(path: &Path) -> Result<fs::File, String> {
    fs::File::create(path).map_err(|e| format!("create {}: {e}", path.display()))?;
    fs::OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|e| format!("open {}: {e}", path.display()))
}

/// The env var naming the call an eval serves.
pub(crate) const NONCE_VAR: &str = "NONCE";

/// Evaluate a synthesized source string against `engine_state`, its value answered.
pub(crate) fn eval_in_process(
    engine_state: &mut nu::EngineState,
    log_dir: &std::path::Path,
    source: &str,
    cancel: Arc<AtomicBool>,
    persist: bool,
    nonce: Option<&str>,
    host: Option<&HostLink>,
) -> EngineResult<nu::Value> {
    let stdout_file = open_log(&log_dir.join(STDOUT_FILE))?;
    let stderr_file = open_log(&log_dir.join(STDERR_FILE))?;
    let mut stack = nu::Stack::new()
        .stdout_file(stdout_file)
        .stderr_file(stderr_file)
        .capture_all();
    if let Some(nonce) = nonce {
        stack.add_env_var(
            NONCE_VAR.to_string(),
            nu::Value::string(nonce, nu::Span::unknown()),
        );
    }
    engine_state.set_signals(nu::Signals::new(cancel));
    let mut working_set = nu::StateWorkingSet::new(engine_state);
    register_nuapi(&mut working_set, host);
    let files_before = working_set.num_files();
    let block = nu::parse(&mut working_set, None, source.as_bytes(), false);
    if !working_set.parse_errors.is_empty() {
        if let Some(files) = detect_import_cycle(&working_set, files_before) {
            return Err(EngineError::CircularImport { files });
        }
        let msgs: Vec<String> = working_set
            .parse_errors
            .iter()
            .map(|e| nuval::NuValError::from(e).short())
            .collect();
        return Err(format!("parse errors: {}", msgs.join("; ")).into());
    }
    if !working_set.compile_errors.is_empty() {
        let msgs: Vec<String> = working_set
            .compile_errors
            .iter()
            .map(|e| nuval::NuValError::from(e).short())
            .collect();
        return Err(format!("compile errors: {}", msgs.join("; ")).into());
    }
    let delta = working_set.render();
    engine_state
        .merge_delta(delta)
        .map_err(|e| format!("merge_delta: {}", nuval::NuValError::from(&e).short()))?;
    let pipeline = nu::eval_block::<nu::WithoutDebug>(
        engine_state,
        &mut stack,
        &block,
        nu::PipelineData::Empty,
    )
    .map_err(|e| format!("eval: {}", nuval::NuValError::from(&e).short()))?;
    let value = pipeline
        .body
        .into_value(nu::Span::unknown())
        .map_err(|e| format!("into_value: {}", nuval::NuValError::from(&e).short()))?;
    if persist {
        let _ = stack.remove_env_var(engine_state, NONCE_VAR);
        merge_env_no_chdir(engine_state, &mut stack);
    }
    Ok(value)
}

/// Empty a jobs table a caught panic poisoned, and clear its poison.
fn heal_jobs(jobs: &std::sync::Mutex<nu::Jobs>) {
    if jobs.is_poisoned() {
        *jobs.lock().unwrap_or_else(|e| e.into_inner()) = nu::Jobs::default();
        jobs.clear_poison();
    }
}

/// Generous per-eval stack; the parser is what needs it.
pub const EVAL_STACK_SIZE: usize = 64 * 1024 * 1024;

/// Run one stateless eval on a dedicated, generously-stacked thread.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn eval_stateless(
    mut engine: nu::EngineState,
    cancel: Arc<AtomicBool>,
    log_dir: std::path::PathBuf,
    source: String,
    nonce: String,
    permit: tk::OwnedSemaphorePermit,
    finished: Arc<AtomicBool>,
    host: HostLink,
) -> EngineResult<nu::Value> {
    let (tx, rx) = tk::oneshot::channel();
    let spawned = std::thread::Builder::new()
        .stack_size(EVAL_STACK_SIZE)
        .name("nu-eval".to_string())
        .spawn(move || {
            let _permit = permit;
            let _finish = FinishGuard(finished);
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                eval_in_process(&mut engine, &log_dir, &source, cancel, false, Some(&nonce), Some(&host))
            }));
            let result = match outcome {
                Ok(r) => r,
                Err(_) => {
                    heal_jobs(&engine.jobs);
                    Err("panic during eval (caught)".to_string().into())
                }
            };
            let _ = tx.send(result);
        });
    if let Err(e) = spawned {
        return Err(format!("spawn eval thread: {e}").into());
    }
    match rx.await {
        Ok(r) => r,
        Err(_) => Err("eval thread dropped without result".to_string().into()),
    }
}

/// The persistent stateful engine: one long-lived serial thread.
#[derive(Clone)]
pub(crate) struct StatefulEngine {
    tx: tk::UnboundedSender<StatefulRequest>,
}

struct StatefulRequest {
    log_dir: std::path::PathBuf,
    source: String,
    nonce: String,
    cancel: Arc<AtomicBool>,
    tracker: nu::ThreadJob,
    finished: Arc<AtomicBool>,
    respond: tk::oneshot::Sender<EngineResult<nu::Value>>,
}

impl StatefulEngine {
    /// Spawn the stateful engine thread with the host-owned jobs table.
    pub(crate) fn spawn(
        env_jobs: Arc<std::sync::Mutex<nu::Jobs>>,
        host: HostLink,
    ) -> Self {
        let (tx, mut rx) = tk::unbounded_channel::<StatefulRequest>();
        let _ = std::thread::Builder::new()
            .stack_size(EVAL_STACK_SIZE)
            .name("nu-stateful".to_string())
            .spawn(move || {
                let mut engine = startup(Mode::Stateful);
                engine.jobs = env_jobs.clone();
                while let Some(req) = rx.blocking_recv() {
                    let _finish = FinishGuard(req.finished.clone());
                    engine.current_job.background_thread_job = Some(req.tracker.clone());
                    let outcome = catch_unwind(AssertUnwindSafe(|| {
                        eval_in_process(
                            &mut engine,
                            &req.log_dir,
                            &req.source,
                            req.cancel.clone(),
                            true,
                            Some(&req.nonce),
                            Some(&host),
                        )
                    }));
                    let result = match outcome {
                        Ok(r) => r,
                        Err(_) => {
                            heal_jobs(&env_jobs);
                            engine = startup(Mode::Stateful);
                            engine.jobs = env_jobs.clone();
                            Err("panic during a stateful eval (caught); the stateful session reset"
                                .to_string()
                                .into())
                        }
                    };
                    let _ = req.respond.send(result);
                }
            });
        Self { tx }
    }

    /// Submit one stateful eval to the engine thread and await its result.
    pub(crate) async fn eval(
        &self,
        log_dir: std::path::PathBuf,
        source: String,
        nonce: String,
        cancel: Arc<AtomicBool>,
        tracker: nu::ThreadJob,
        finished: Arc<AtomicBool>,
    ) -> EngineResult<nu::Value> {
        let (respond, rx) = tk::oneshot::channel();
        self.tx
            .send(StatefulRequest {
                log_dir,
                source,
                nonce,
                cancel,
                tracker,
                finished,
                respond,
            })
            .map_err(|_| "the stateful engine thread is gone".to_string())?;
        match rx.await {
            Ok(r) => r,
            Err(_) => Err("the stateful engine dropped the response".to_string().into()),
        }
    }
}
