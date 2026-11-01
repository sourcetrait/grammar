//! mcp's controller: the process's systems started, routed, and stopped in order.
use crate::*;

/// How long the shutdown waits for in-flight requests to let the engine go.
const ENGINE_RELEASE_GRACE: tk::TkDuration = tk::TkDuration::from_secs(1);

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
fn held<T>(slot: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    slot.lock().unwrap_or_else(|e| e.into_inner())
}

/// The operator and the observer that feeds it, the serve path's supervision.
struct Supervision {
    operator: Arc<OperatorHandle>,
    observer: tk::JoinHandle<()>,
}

/// The process's systems, with the namespace's lock held for their life.
pub(crate) struct McpController {
    /// The runtime config the process resolved, which every system starts on.
    pub(crate) config: Arc<Config>,
    /// The Channel's and the peer links' systems, and the open's hub.
    pub(crate) face: ChannelFace,
    /// The engine's system, none before its start and once the shutdown takes it.
    engine: std::sync::Mutex<Option<Arc<EngineHandle>>>,
    /// The remote engines' system, none before its start or once the shutdown takes it.
    remote: std::sync::Mutex<Option<Arc<RemoteHandle>>>,
    /// The operator and its observer, when the controller supervises.
    supervision: std::sync::Mutex<Option<Supervision>>,
    /// The namespace's lock, released once the engine and the operator stop.
    lock: std::sync::Mutex<Option<NamespaceLock>>,
    /// Set as the shutdown starts, so the systems' ends it asks for go unsaid.
    stopping: Arc<AtomicBool>,
    /// Held across the shutdown, so a second caller waits for the first's.
    stopped: tk::AsyncMutex<bool>,
    /// The signal sweep, which an ordinary end stands down.
    sweep: std::sync::Mutex<Option<tk::JoinHandle<()>>>,
}

impl McpController {
    /// The systems started in order, the face's first, `lock` held for their life.
    pub(crate) async fn start(
        config: Arc<Config>,
        supervise: bool,
        lock: Option<NamespaceLock>,
    ) -> Result<Arc<Self>, String> {
        let face = Self::start_face(&config).await?;
        let controller = Self::of(config, face, lock);
        let remote = controller.start_engine(supervise).await?;
        controller.start_remote(remote).await?;
        Ok(Arc::new(controller))
    }

    /// The Channel's and the peer links' systems, on the process's identity.
    async fn start_face(config: &Arc<Config>) -> Result<ChannelFace, String> {
        let (nonce_gen, mcp_nom) = process_identity();
        ChannelFace::start(config.clone(), nonce_gen, mcp_nom).await
    }

    /// A controller over `face`, no other system started yet.
    fn of(
        config: Arc<Config>,
        face: ChannelFace,
        lock: Option<NamespaceLock>,
    ) -> Self {
        Self {
            config,
            face,
            engine: std::sync::Mutex::new(None),
            remote: std::sync::Mutex::new(None),
            supervision: std::sync::Mutex::new(None),
            lock: std::sync::Mutex::new(lock),
            stopping: Arc::new(AtomicBool::new(false)),
            stopped: tk::AsyncMutex::new(false),
            sweep: std::sync::Mutex::new(None),
        }
    }

    /// The engine's system, its asks routed here, then the supervision if asked.
    ///
    /// Returns the remote engines' handler, the same route and policy the engine's take.
    async fn start_engine(
        &self,
        supervise: bool,
    ) -> Result<RemoteRoute, String> {
        let mcp_nom = self.face.mcp_nom.as_str();
        let log = emergency_log_path(&self.config.cache_home(), mcp_nom);
        let policy = EmergencyPolicy::new(self.face.channel.clone(), mcp_nom, log);
        if supervise {
            self.face.hub.install_policy(policy.clone());
        }
        let route = HostRoute {
            channel: self.face.channel.clone(),
            peer: self.face.peer.clone(),
            config: self.config.clone(),
            stopping: self.stopping.clone(),
        };
        let remote = RemoteRoute {
            route: route.clone(),
            policy: policy.clone(),
        };
        let engine = Arc::new(EngineHandle::start(route).await.map_err(|e| e.to_string())?);
        if supervise {
            let supervision = self.supervise(engine.clone(), policy).await?;
            *held(&self.supervision) = Some(supervision);
        }
        *held(&self.engine) = Some(engine);
        Ok(remote)
    }

    /// The remote engines' system on the config's engines, `route` answering asks.
    async fn start_remote(
        &self,
        route: RemoteRoute,
    ) -> Result<(), String> {
        let paths = RemoteSysPaths::new(self.config.data_home(), self.config.cache_home());
        let params = RemoteSysParams {
            endpoints: self.config.engines.clone(),
            limits: LIMITS_DEFAULT,
        };
        let remote = RemoteHandle::start(paths, params, route).await?;
        *held(&self.remote) = Some(Arc::new(remote));
        Ok(())
    }

    /// The operator on the namespace's homes, its emergencies the policy's, observed.
    async fn supervise(
        &self,
        engine: Arc<EngineHandle>,
        policy: EmergencyPolicy,
    ) -> Result<Supervision, String> {
        let paths = OperatorSysPaths::new(self.config.data_home(), self.config.cache_home());
        let params = OperatorSysParams {
            test: self.config.test,
            supervisor: self.config.supervisor,
        };
        let watch = OperatorWatch {
            policy,
            stopping: self.stopping.clone(),
        };
        let operator = Arc::new(OperatorHandle::start(paths, params, watch).await?);
        let observer = spawn_observer(engine, operator.clone(), self.face.channel.clone());
        Ok(Supervision { operator, observer })
    }

    /// The engine's system, or the failure a stopped one answers.
    fn engine(&self) -> EngineResult<Arc<EngineHandle>> {
        held(&self.engine).clone().ok_or_else(|| EngineError::Internal {
            phase: "mcp::engine".to_string(),
            reason: "the engine's system has stopped".to_string(),
        })
    }

    /// Sends a request to the engine and awaits its answer.
    pub(crate) async fn request<T>(
        &self,
        req: T,
    ) -> EngineResult<T::ResponseType>
    where
        T: subsys::Request<EngineSystem>,
        T::ResponseType: TryFrom<FromEngine, Error = subsys::SubsysError>,
    {
        self.engine()?.request(req).await
    }

    /// Sends a command to the local engine and awaits its answer.
    pub(crate) async fn control(
        &self,
        command: EngineCommand,
    ) -> EngineResult<ControlAnswer> {
        self.engine()?.control(command).await
    }

    /// Sends a request to the remote engine `alias` names and awaits its response.
    pub(crate) async fn remote(
        &self,
        alias: &str,
        request: EngineRequest,
    ) -> Result<EngineResponse, TransportError> {
        let Some(remote) = held(&self.remote).clone() else {
            return Err(TransportError::Unavailable {
                reason: "the remote engines' system has stopped".to_string(),
            });
        };
        remote.request(alias, request).await
    }

    /// Sweep on TERM, INT, or HUP: the systems stopped, then the process's exit.
    pub(crate) fn sweep_signals(self: &Arc<Self>) {
        let controller = self.clone();
        let sweep = spawn_signal_sweep(move || async move {
            held(&controller.sweep).take();
            controller.stop().await;
        });
        *held(&self.sweep) = sweep;
    }

    /// The ordinary end: the sweep stood down, then the systems stopped in order.
    pub(crate) async fn shutdown(&self) {
        if let Some(sweep) = held(&self.sweep).take() {
            sweep.abort();
        }
        self.stop().await;
    }

    /// The sessions, the Channel, and the engine stopped, the lock released, once.
    async fn stop(&self) {
        let mut stopped = self.stopped.lock().await;
        if *stopped {
            return;
        }
        self.stopping.store(true, Ordering::SeqCst);
        self.stop_remote().await;
        self.face.closer().close_for_shutdown().await;
        self.stop_engine().await;
        drop(held(&self.lock).take());
        *stopped = true;
    }

    /// Every remote session closed, its requests let go, then the system stopped.
    async fn stop_remote(&self) {
        let Some(remote) = held(&self.remote).take() else {
            return;
        };
        remote.close().await;
        if let Err(e) = RemoteHandle::stop(remote).await {
            eprintln!("{e}");
        }
    }

    /// The observer, then the engine's calls, the operator, and the engine stopped.
    async fn stop_engine(&self) {
        let supervision = held(&self.supervision).take();
        let operator = match supervision {
            Some(Supervision { operator, observer }) => {
                observer.abort();
                let _ = observer.await;
                Some(operator)
            }
            None => None,
        };
        let Some(mut engine) = held(&self.engine).take() else {
            return;
        };
        if let Err(e) = engine.control(EngineCommand::Teardown).await {
            eprintln!("grammar: the engine's teardown: {e}");
        }
        if let Some(operator) = operator
            && let Ok(operator) = Arc::try_unwrap(operator)
            && let Err(e) = operator.stop().await
        {
            eprintln!("{e}");
        }
        let deadline = Instant::now() + ENGINE_RELEASE_GRACE;
        loop {
            match Arc::try_unwrap(engine) {
                Ok(handle) => {
                    if let Err(e) = handle.stop().await {
                        eprintln!("grammar: the engine's stop: {e}");
                    }
                    return;
                }
                Err(shared) if Instant::now() < deadline => {
                    engine = shared;
                    tk::sleep(tk::TkDuration::from_millis(10)).await;
                }
                Err(_) => return,
            }
        }
    }
}
