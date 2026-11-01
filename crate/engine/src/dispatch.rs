use crate::*;

/// A call's timeout when it names none, in milliseconds.
pub(crate) const DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// Pre-clone depth N for the stateless executor's ready buffer.
pub(crate) const READY_POOL_TARGET: usize = 1;

/// Write an `execute` call's definition beside its logs, the handle `rerun` reads.
fn write_definition(
    log_dir: &Path,
    definition: &str,
) {
    let path = log_dir.join(DEFINITION_FILE);
    if let Err(e) = fs::write(&path, definition.as_bytes()) {
        eprintln!("grammar: definition cache write failed at {}: {e}", path.display());
    }
}

pub(crate) struct InFlightEntry {
    pub tool: &'static str,
    pub started_at: u64,
    pub args: NuVal,
    pub kind: InFlightKind,
    /// This eval's interrupt flag - the `Signals` handle its thread polls.
    pub cancel: Arc<AtomicBool>,
    /// This eval's external-child tracker, holding Arc-shared pids.
    pub tracker: nu::ThreadJob,
    /// Liveness: the eval thread flips this on exit via a Drop guard.
    pub finished: Arc<AtomicBool>,
}

pub(crate) enum InFlightKind {
    Execute { lane: Lane },
    Rerun { source_nonce: String, lane: Lane },
    Call { path: String },
}

impl InFlightKind {
    /// The lane the call's eval runs on.
    pub(crate) fn lane(&self) -> Lane {
        match self {
            Self::Execute { lane } | Self::Rerun { lane, .. } => *lane,
            Self::Call { .. } => Lane::Stateless,
        }
    }
}

/// The calls in flight, keyed by nonce.
pub(crate) type InFlight = Arc<tk::AsyncMutex<HashMap<String, InFlightEntry>>>;

/// A cancelled engine thread watched for a hang.
#[derive(Clone)]
pub(crate) struct HungWatch {
    pub nonce: String,
    pub tool: &'static str,
    pub lane: Lane,
    pub started_at: u64,
    pub cancelled_at: u64,
    pub finished: Arc<AtomicBool>,
}

/// The cancelled-thread registry the health report reads. Keyed by nonce.
pub(crate) type HungRegistry = Arc<std::sync::Mutex<HashMap<String, HungWatch>>>;

/// Record a cancelled-but-maybe-alive engine thread for the watch.
pub(crate) fn register_hung(
    registry: &HungRegistry,
    watch: HungWatch,
) {
    let mut map = registry.lock().unwrap_or_else(|e| e.into_inner());
    map.entry(watch.nonce.clone()).or_insert(watch);
}

/// The watched threads still running, the finished ones pruned as they're read.
pub(crate) fn hung_snapshot(registry: &HungRegistry) -> Vec<HungEval> {
    let mut map = registry.lock().unwrap_or_else(|e| e.into_inner());
    map.retain(|_, w| !w.finished.load(Ordering::SeqCst));
    map.values()
        .map(|w| HungEval {
            nonce: w.nonce.clone(),
            tool: w.tool.to_string(),
            lane: w.lane,
            started_at: w.started_at,
            cancelled_at: w.cancelled_at,
        })
        .collect()
}

/// What one call's dispatch needs to know of it.
pub(crate) struct Dispatch<'a> {
    pub nonce: &'a datum::NoncePair,
    pub source: String,
    pub tool: &'static str,
    pub args: NuVal,
    pub kind: InFlightKind,
    pub definition: Option<&'a str>,
    pub timeout_ms: Option<u64>,
}

pub(crate) async fn dispatch_pooled(
    core: &EngineCore,
    log_kind: CacheKind,
    call: Dispatch<'_>,
) -> EngineResult<nu::Value> {
    let Dispatch {
        nonce,
        source,
        tool,
        args,
        kind,
        definition,
        timeout_ms,
    } = call;
    let log_dir = cache_dir(log_kind, nonce);
    fs::create_dir_all(&log_dir).map_err(|e| EngineError::Internal {
        phase: "dispatch_pooled::create_log_dir".to_string(),
        reason: format!("create_dir_all {}: {e}", log_dir.display()),
    })?;
    if let Some(definition) = definition {
        write_definition(&log_dir, definition);
    }
    core.executor.refresh_base_if_stale();
    let effective_timeout = timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    let budget = tk::TkDuration::from_millis(effective_timeout);
    let dispatched_at = Instant::now();
    let semaphore = core.executor.semaphore();
    let permit = match tk::timeout(budget, semaphore.clone().acquire_owned()).await {
        Ok(Ok(permit)) => permit,
        Ok(Err(e)) => {
            return Err(EngineError::Dispatch {
                reason: format!("eval semaphore: {e}"),
            });
        }
        Err(_) => {
            let cap = eval_concurrency_cap();
            let held = cap.saturating_sub(semaphore.available_permits());
            return Err(EngineError::Dispatch {
                reason: format!(
                    "no eval permit within {effective_timeout} ms; {held} of {cap} permits held",
                ),
            });
        }
    };
    let mut engine = core.executor.take_clone();
    let cancel = Arc::new(AtomicBool::new(false));
    let started_at = now_millis();
    let finished = Arc::new(AtomicBool::new(false));
    let tracker = make_tracker(cancel.clone());
    engine.current_job.background_thread_job = Some(tracker.clone());
    register_in_flight(
        &core.in_flight,
        nonce,
        InFlightEntry {
            tool,
            started_at,
            args,
            kind,
            cancel: cancel.clone(),
            tracker: tracker.clone(),
            finished: finished.clone(),
        },
    )
    .await;
    let _flight_cleanup = InFlightCleanup {
        map: core.in_flight.clone(),
        key: nonce.as_str().to_string(),
    };
    let eval_fut = eval_stateless(
        engine,
        cancel.clone(),
        log_dir,
        source,
        nonce.as_str().to_string(),
        permit,
        finished.clone(),
        core.host.clone(),
    );
    let remaining = budget.saturating_sub(dispatched_at.elapsed());
    match tk::timeout(remaining, eval_fut).await {
        Ok(result) => result,
        Err(_) => {
            cancel.store(true, Ordering::SeqCst);
            tree_kill(&tracker.collect_pids());
            register_hung(
                &core.hung,
                HungWatch {
                    nonce: nonce.as_str().to_string(),
                    tool,
                    lane: Lane::Stateless,
                    started_at,
                    cancelled_at: now_millis(),
                    finished: finished.clone(),
                },
            );
            Err(EngineError::Timeout {
                timeout_ms: effective_timeout,
            })
        }
    }
}

pub(crate) async fn dispatch_stateful(
    core: &EngineCore,
    call: Dispatch<'_>,
) -> EngineResult<nu::Value> {
    let Dispatch {
        nonce,
        source,
        tool,
        args,
        kind,
        definition,
        timeout_ms,
    } = call;
    let log_dir = cache_dir(CacheKind::Execute, nonce);
    fs::create_dir_all(&log_dir).map_err(|e| EngineError::Internal {
        phase: "dispatch_stateful::create_log_dir".to_string(),
        reason: format!("create_dir_all {}: {e}", log_dir.display()),
    })?;
    if let Some(definition) = definition {
        write_definition(&log_dir, definition);
    }
    let engine = {
        let mut guard = core.stateful.lock().await;
        if guard.is_none() {
            *guard = Some(StatefulEngine::spawn(core.env_jobs.clone(), core.host.clone()));
        }
        guard
            .as_ref()
            .expect("stateful engine present after spawn")
            .clone()
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let tracker = make_tracker(cancel.clone());
    let started_at = now_millis();
    let finished = Arc::new(AtomicBool::new(false));
    register_in_flight(
        &core.in_flight,
        nonce,
        InFlightEntry {
            tool,
            started_at,
            args,
            kind,
            cancel: cancel.clone(),
            tracker: tracker.clone(),
            finished: finished.clone(),
        },
    )
    .await;
    let _flight_cleanup = InFlightCleanup {
        map: core.in_flight.clone(),
        key: nonce.as_str().to_string(),
    };
    let effective_timeout = timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    let eval_fut = engine.eval(
        log_dir,
        source,
        nonce.as_str().to_string(),
        cancel.clone(),
        tracker.clone(),
        finished.clone(),
    );
    match tk::timeout(tk::TkDuration::from_millis(effective_timeout), eval_fut).await {
        Ok(result) => result,
        Err(_) => {
            cancel.store(true, Ordering::SeqCst);
            tree_kill(&tracker.collect_pids());
            register_hung(
                &core.hung,
                HungWatch {
                    nonce: nonce.as_str().to_string(),
                    tool,
                    lane: Lane::Stateful,
                    started_at,
                    cancelled_at: now_millis(),
                    finished: finished.clone(),
                },
            );
            Err(EngineError::Timeout {
                timeout_ms: effective_timeout,
            })
        }
    }
}

async fn register_in_flight(
    in_flight: &InFlight,
    nonce: &datum::NoncePair,
    entry: InFlightEntry,
) {
    let mut map = in_flight.lock().await;
    map.insert(nonce.as_str().to_string(), entry);
}

struct InFlightCleanup {
    map: InFlight,
    key: String,
}

impl Drop for InFlightCleanup {
    fn drop(&mut self) {
        let map = self.map.clone();
        let key = std::mem::take(&mut self.key);
        tk::spawn(async move {
            let mut g = map.lock().await;
            g.remove(&key);
        });
    }
}

pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Clean-shutdown teardown: cancel every in-flight eval and reap its tree.
pub(crate) async fn teardown_all_in_flight(core: &EngineCore) {
    let pids: Vec<u32> = {
        let map = core.in_flight.lock().await;
        let mut all = Vec::new();
        for entry in map.values() {
            entry.cancel.store(true, Ordering::SeqCst);
            all.extend(entry.tracker.collect_pids());
        }
        all
    };
    tree_kill(&pids);
    sweep_env_jobs(&core.env_jobs);
    kill_plugin_subprocesses();
}

/// Kill every background job in the host-owned table and reap their trees.
pub(crate) fn sweep_env_jobs(env_jobs: &Arc<std::sync::Mutex<nu::Jobs>>) {
    let tracked: Vec<u32> = {
        let mut jobs = env_jobs.lock().unwrap_or_else(|e| e.into_inner());
        let tracked = jobs
            .iter()
            .filter_map(|(_, job)| match job {
                nu::Job::Thread(thread_job) => Some(thread_job.collect_pids()),
                _ => None,
            })
            .flatten()
            .collect();
        let _ = jobs.kill_all();
        tracked
    };
    tree_kill(&tracked);
}
