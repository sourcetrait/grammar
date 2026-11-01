//! The worker's controller: its engine's and operator's systems, their asks routed.
use crate::*;

/// How long the shutdown waits for in-flight requests to let the engine go.
const ENGINE_RELEASE_GRACE: std::time::Duration = std::time::Duration::from_secs(1);

/// A system's status change, logged unless a shutdown is running.
fn log_status(
    component: &str,
    sub: subsys::FromSub,
    stopping: &AtomicBool,
) {
    if stopping.load(Ordering::SeqCst) {
        return;
    }
    if let subsys::FromSub::StatusChange(change) = sub {
        eprintln!("grammard: the {component}'s system: {:?}", change.msg().0);
    }
}

/// A system's end its control did not ask for, logged unless a shutdown is running.
fn log_closed(
    component: &str,
    stopping: &AtomicBool,
) {
    if !stopping.load(Ordering::SeqCst) {
        eprintln!("grammard: the {component}'s system ended unasked");
    }
}

/// A slot's lock, a poisoned one taken as it stands.
fn held<T>(slot: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    slot.lock().unwrap_or_else(|e| e.into_inner())
}

/// The operator and the observer that feeds it.
struct Supervision {
    operator: Arc<OperatorHandle>,
    observer: tk::JoinHandle<()>,
}

/// The worker's systems, under the one controller its process owns.
pub struct GrammardController {
    /// The engine's system, none once the shutdown takes it.
    engine: std::sync::Mutex<Option<Arc<EngineHandle>>>,
    /// The operator and its observer.
    supervision: std::sync::Mutex<Option<Supervision>>,
    /// The session's MCP, for the engine's asks and the operator's emergencies.
    asker: Arc<Asker>,
    /// Set as the shutdown starts, so the systems' ends it asks for go unsaid.
    stopping: Arc<AtomicBool>,
}

impl fmt::Debug for GrammardController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GrammardController").finish_non_exhaustive()
    }
}

impl GrammardController {
    /// The engine's system, its asks routed, then the operator observing it.
    pub(crate) async fn start(config: &InstanceConfig) -> DaemonResult<Self> {
        let failed = |reason: String| DaemonError::EngineInit { reason };
        let asker = Arc::new(Asker::default());
        let stopping = Arc::new(AtomicBool::new(false));
        let route = WorkerRoute {
            asker: asker.clone(),
            supervisor: config.supervisor,
            stopping: stopping.clone(),
        };
        let engine = Arc::new(EngineHandle::start(route).await.map_err(|e| failed(e.to_string()))?);
        let paths = OperatorSysPaths::new(config.paths.data.clone(), config.paths.cache.clone());
        let params = OperatorSysParams {
            test: false,
            supervisor: config.supervisor,
        };
        let watch = WorkerWatch {
            asker: asker.clone(),
            stopping: stopping.clone(),
        };
        let operator = Arc::new(OperatorHandle::start(paths, params, watch).await.map_err(failed)?);
        let observer = spawn_observer(engine.clone(), operator.clone(), asker.clone());
        Ok(Self {
            engine: std::sync::Mutex::new(Some(engine)),
            supervision: std::sync::Mutex::new(Some(Supervision { operator, observer })),
            asker,
            stopping,
        })
    }

    /// The engine's system, or the failure a stopped one answers.
    pub(crate) fn engine(&self) -> EngineResult<Arc<EngineHandle>> {
        held(&self.engine).clone().ok_or_else(|| EngineError::Internal {
            phase: "grammard::engine".to_string(),
            reason: "the engine's system has stopped".to_string(),
        })
    }

    /// The way to the session's MCP.
    pub(crate) fn asker(&self) -> &Arc<Asker> {
        &self.asker
    }

    /// Every call in flight cancelled and its process tree reaped.
    pub(crate) async fn teardown(&self) {
        if let Ok(engine) = self.engine()
            && let Err(e) = engine.control(EngineCommand::Teardown).await
        {
            eprintln!("grammard: the engine's teardown: {e}");
        }
    }

    /// The observer, then the engine's calls, the operator, and the engine stopped.
    pub(crate) async fn shutdown(&self) {
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
            eprintln!("grammard: {e}");
        }
        let Some(mut engine) = held(&self.engine).take() else {
            return;
        };
        let deadline = std::time::Instant::now() + ENGINE_RELEASE_GRACE;
        loop {
            match Arc::try_unwrap(engine) {
                Ok(handle) => {
                    if let Err(e) = handle.stop().await {
                        eprintln!("grammard: the engine's stop: {e}");
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

/// The way to the session's MCP: its outbox while attached, and the asks waiting.
#[derive(Debug, Default)]
pub(crate) struct Asker {
    out: std::sync::Mutex<Option<Outbox>>,
    next: AtomicU64,
    pending: std::sync::Mutex<HashMap<u64, tk::oneshot::Sender<HostAnswer>>>,
}

impl Asker {
    /// A session's outbox attached, the asks after it carried on it.
    pub(crate) fn attach(&self, out: Outbox) {
        *held(&self.out) = Some(out);
    }

    /// The session's outbox let go, each ask still waiting refused.
    pub(crate) fn detach(&self) {
        held(&self.out).take();
        held(&self.pending).clear();
    }

    /// Whether a session is attached.
    pub(crate) fn attached(&self) -> bool {
        held(&self.out).is_some()
    }

    /// The MCP's answer to the ask under `id`, to the eval waiting on it.
    pub(crate) fn answer(&self, id: u64, answer: HostAnswer) {
        if let Some(waiter) = held(&self.pending).remove(&id) {
            let _ = waiter.send(answer);
        }
    }

    /// An eval's ask of the MCP, refused without a session or an answer in time.
    pub(crate) async fn ask(&self, ask: SessionAsk) -> HostAnswer {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = tk::oneshot::channel();
        {
            let out = held(&self.out);
            let Some(out) = out.as_ref() else {
                return HostAnswer::Refused("no session carries the ask".to_string());
            };
            held(&self.pending).insert(id, tx);
            if let Err(unsent) = out.send(&WorkerFrame::Ask { id, ask }) {
                held(&self.pending).remove(&id);
                return HostAnswer::Refused(unsent.to_string());
            }
        }
        match tk::timeout(ASK_DEADLINE, rx).await {
            Ok(Ok(answer)) => answer,
            Ok(Err(_)) => HostAnswer::Refused("the session ended before the ask's answer".to_string()),
            Err(_) => {
                held(&self.pending).remove(&id);
                HostAnswer::Refused(format!("no answer to the ask in {} s", ASK_DEADLINE.as_secs()))
            }
        }
    }

    /// An emergency to the session's MCP, where one is attached.
    pub(crate) fn emergency(&self, emergency: Emergency) {
        if let Some(out) = held(&self.out).as_ref() {
            let _ = out.send(&WorkerFrame::Emergency(emergency));
        }
    }
}

/// The engine's host asks: a pin held here, every other one asked of the MCP.
#[derive(Clone)]
struct WorkerRoute {
    asker: Arc<Asker>,
    /// The worker's own supervisor lines, which a config answer carries.
    supervisor: SupervisorConfig,
    stopping: Arc<AtomicBool>,
}

impl WorkerRoute {
    /// An eval's ask, answered here or by the session's MCP.
    async fn answer(
        &self,
        ask: HostAsk,
    ) -> HostAnswer {
        match ask {
            HostAsk::Pin(PinAsk { key, pid, value }) => match pin(&key, pid, value) {
                Ok(()) => HostAnswer::Pinned,
                Err(reason) => HostAnswer::Refused(reason),
            },
            HostAsk::Config => match self.asker.ask(SessionAsk::Config).await {
                HostAnswer::Config(record) => {
                    HostAnswer::Config(with_supervisor(record, effective_supervisor(self.supervisor)))
                }
                other => other,
            },
            HostAsk::ChannelSend(data) => self.asker.ask(SessionAsk::ChannelSend(data)).await,
            HostAsk::Stopped(stopped) => self.asker.ask(SessionAsk::Stopped(stopped)).await,
            HostAsk::RemoteSend(data) => match tk::spawn_blocking(move || prepare_send(data)).await {
                Ok(Ok(prepared)) => self.asker.ask(SessionAsk::RemoteSend(prepared)).await,
                Ok(Err(reason)) => HostAnswer::Refused(reason),
                Err(e) => HostAnswer::Refused(format!("the send's files: {e}")),
            },
        }
    }
}

impl subsys::Handler<EngineSystem> for WorkerRoute {
    fn on_packet(&mut self, pkt: subsys::Packet<FromEngine>) -> impl Future<Output = Option<subsys::Packet<FromEngine>>> + 'static + Send {
        std::future::ready(Some(pkt))
    }

    fn on_request(&mut self, pkt: subsys::Packet<FromEngine>) -> impl Future<Output = Option<ToEngine>> + 'static + Send {
        let route = self.clone();
        async move {
            let FromEngine::Host(ask) = pkt.take_msg() else {
                return None;
            };
            Some(ToEngine::Host(route.answer(ask).await))
        }
    }

    fn on_sub(&mut self, sub: subsys::FromSub) -> impl Future<Output = ()> + Send {
        log_status("engine", sub, &self.stopping);
        std::future::ready(())
    }

    fn on_closed(&mut self) -> impl Future<Output = ()> + Send {
        log_closed("engine", &self.stopping);
        std::future::ready(())
    }
}

/// The MCP's config record with the worker's supervisor lines in place of its own.
fn with_supervisor(
    record: NuVal,
    supervisor: SupervisorConfig,
) -> NuVal {
    let lines = NuVal::Record(vec![
        ("cpu_warn_fraction".to_string(), NuVal::Float(supervisor.cpu_warn_fraction)),
        ("ram_warn_fraction".to_string(), NuVal::Float(supervisor.ram_warn_fraction)),
        (
            "vram_warn_headroom_mib".to_string(),
            NuVal::Int(i64::try_from(supervisor.vram_warn_headroom_mib).unwrap_or(i64::MAX)),
        ),
        ("disk_warn_fraction".to_string(), NuVal::Float(supervisor.disk_warn_fraction)),
    ]);
    match record {
        NuVal::Record(mut fields) => {
            match fields.iter_mut().find(|(key, _)| key == "supervisor") {
                Some((_, value)) => *value = lines,
                None => fields.push(("supervisor".to_string(), lines)),
            }
            NuVal::Record(fields)
        }
        other => other,
    }
}

/// The controller's handler of the operator: its emergencies to the session's MCP.
struct WorkerWatch {
    asker: Arc<Asker>,
    stopping: Arc<AtomicBool>,
}

impl subsys::Handler<OperatorSystem> for WorkerWatch {
    fn on_packet(&mut self, pkt: subsys::Packet<FromOperatorSys>) -> impl Future<Output = Option<subsys::Packet<FromOperatorSys>>> + 'static + Send {
        let FromOperatorSys::Emergency(emergency) = pkt.take_msg();
        eprintln!("grammard: an emergency: {emergency:?}");
        self.asker.emergency(emergency);
        std::future::ready(None)
    }

    fn on_sub(&mut self, sub: subsys::FromSub) -> impl Future<Output = ()> + Send {
        log_status("operator", sub, &self.stopping);
        std::future::ready(())
    }

    fn on_closed(&mut self) -> impl Future<Output = ()> + Send {
        log_closed("operator", &self.stopping);
        std::future::ready(())
    }
}

/// The engine's health, or none when it doesn't answer within a sample's interval.
async fn engine_health(engine: &EngineHandle) -> Option<EngineHealth> {
    match tk::timeout(SAMPLE_INTERVAL, engine.control(EngineCommand::Health)).await {
        Ok(Ok(ControlAnswer::Health(health))) => Some(health),
        _ => None,
    }
}

/// Each interval, the operator given the engine's health and a session's presence.
fn spawn_observer(
    engine: Arc<EngineHandle>,
    operator: Arc<OperatorHandle>,
    asker: Arc<Asker>,
) -> tk::JoinHandle<()> {
    tk::spawn(async move {
        loop {
            tk::sleep(SAMPLE_INTERVAL).await;
            let observation = Observation {
                engine: engine_health(&engine).await,
                listening: asker.attached(),
            };
            if let Err(e) = operator.observe(observation).await {
                eprintln!("grammard: {e}");
                return;
            }
        }
    })
}
