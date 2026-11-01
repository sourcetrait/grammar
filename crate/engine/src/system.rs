use crate::*;

/// The engine as a subsys system: each request answered from a task of its own.
pub struct EngineSystem {
    inner: subsys::InnerSystem<Self>,
    core: Arc<EngineCore>,
}

/// The system's parameters; the engine takes its own through `init`.
#[cereal::derived(Data, Copy, Eq)]
#[derive(Default)]
pub struct EngineSysParams;

impl subsys::Params for EngineSysParams {}

/// The system's configuration, which holds nothing.
#[derive(Default, Debug, Clone, ser::Serialize, ser::Deserialize)]
pub struct EngineSysConfig;

impl subsys::Config for EngineSysConfig {}

/// The namespace's homes as the system's paths.
#[derive(Debug, Clone)]
pub struct EngineSysPaths {
    data_home: PathBuf,
    cache_home: PathBuf,
}

impl EngineSysPaths {
    /// The paths of the namespace `init` gave the engine.
    pub fn of(params: &EngineParams) -> Self {
        Self {
            data_home: params.data_home.clone(),
            cache_home: params.cache_home.clone(),
        }
    }
}

impl agnostic::AppPaths for EngineSysPaths {
    fn cache_dir(&self) -> PathBuf {
        self.cache_home.clone()
    }

    fn config_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn config_secrets_dir(&self) -> PathBuf {
        keypair_dir()
    }

    fn data_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn state_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn state_secrets_dir(&self) -> PathBuf {
        keypair_dir()
    }

    fn runtime_dir(&self) -> PathBuf {
        self.cache_home.clone()
    }
}

impl agnostic::HasAppPaths for EngineSysPaths {
    fn app_paths(&self) -> &impl agnostic::AppPaths {
        self
    }
}

impl subsys::System for EngineSystem {
    const CHANNEL_SIZE: usize = 100;

    type Paths = EngineSysPaths;
    type Params = EngineSysParams;
    type Config = EngineSysConfig;
    type ToSys = ToEngine;
    type FromSys = FromEngine;
    type Flow = subsys::StdFlow;

    fn inner(&self) -> &subsys::InnerSystem<Self> { &self.inner }
    fn inner_mut(&mut self) -> &mut subsys::InnerSystem<Self> { &mut self.inner }

    async fn init(inner: subsys::InnerSystem<Self>, _params: Self::Params) -> subsys::SubsysResult<Self> {
        let host = HostLink(inner.requester());
        let core = EngineCore::start(host).await?;
        Ok(Self {
            inner,
            core: Arc::new(core),
        })
    }

    async fn run(mut self) -> subsys::RunResult {
        let result = loop {
            let flow = match self.inner.channel.rx.recv().await {
                Some(msg) => self.handle_channel_recv(msg).await,
                None => break subsys::SUCCESS,
            };
            match flow {
                Ok(subsys::Flow::Control(subsys::Control::Stop { .. })) => break subsys::SUCCESS,
                Ok(_) => {}
                Err(e) => break Err(e),
            }
        };
        self.done(result).await
    }

    async fn on_channel_recv(&mut self, pkt: subsys::Packet<ToEngine>) -> subsys::FlowResult<Self::Flow> {
        let (id, _nature, msg) = pkt.into_tuple();
        let core = self.core.clone();
        let tx = self.inner.channel.tx.clone();
        match msg {
            ToEngine::Wire(msg) => {
                tk::spawn(async move {
                    let answer = FromEngine::Wire(answer(core, msg).await);
                    let response = subsys::Packet::response(id, answer);
                    let _ = tx.send(subsys::MsgFromSys::Packet(response)).await;
                });
            }
            ToEngine::Purview(req) => {
                tk::spawn(async move {
                    let shape = req.clone();
                    let answer = match tk::spawn_blocking(move || purview_answer(req)).await {
                        Ok(answer) => answer,
                        Err(e) => purview_failed(
                            &shape,
                            RuntimeError::Internal {
                                phase: "purview".to_string(),
                                reason: format!("the request's task failed: {e}"),
                            },
                        ),
                    };
                    let response = subsys::Packet::response(id, FromEngine::Purview(answer));
                    let _ = tx.send(subsys::MsgFromSys::Packet(response)).await;
                });
            }
            ToEngine::Host(_) => {}
        }
        Ok(subsys::Flow::Continue)
    }

    async fn on_stop(&mut self, _halt: bool) -> subsys::RunResult {
        teardown_all_in_flight(&self.core).await;
        subsys::SUCCESS
    }

    async fn on_resume(&mut self) -> subsys::SysResult<bool> {
        Ok(true)
    }
}

/// A request's answer, its work run on a task of its own.
async fn answer(
    core: Arc<EngineCore>,
    msg: ToEngineSys,
) -> FromEngineSys {
    match msg {
        ToEngineSys::Execute(req) => {
            let nonce = req.nonce;
            let result = contained(serve_execute(core, req)).await;
            FromEngineSys::Value(ValueAnswer { nonce, result })
        }
        ToEngineSys::Rerun(req) => {
            let nonce = req.nonce;
            let result = contained(serve_rerun(core, req)).await;
            FromEngineSys::Value(ValueAnswer { nonce, result })
        }
        ToEngineSys::Call(req) => {
            let nonce = req.nonce;
            let result = contained(serve_call(core, req)).await;
            FromEngineSys::Value(ValueAnswer { nonce, result })
        }
        ToEngineSys::Control(req) => FromEngineSys::Control(contained(serve_control(core, req)).await),
    }
}

/// A purview request's answer, its store read and written under the equip lock.
fn purview_answer(req: PurviewRequest) -> PurviewAnswer {
    match req {
        PurviewRequest::List(PurviewListRequest) => PurviewAnswer::Listed(purview_list()),
        PurviewRequest::Configure(PurviewConfigureRequest { id, patterns }) => {
            PurviewAnswer::Rows(purview_configure(&id, &patterns))
        }
        PurviewRequest::Resolve(PurviewResolveRequest { ids }) => {
            PurviewAnswer::Resolved(purview_resolve(&ids))
        }
        PurviewRequest::Rig(PurviewRigRequest {
            rig,
            installed,
            current,
        }) => PurviewAnswer::Rows(match installed {
            true if rig_dir(&rig).exists() => purview_add_rig(&rig, &current),
            true => load_purviews().map(Option::unwrap_or_default),
            false => purview_remove_rig(&rig),
        }),
    }
}

/// A purview request's failure, answered in its request's own kind.
fn purview_failed(
    req: &PurviewRequest,
    error: RuntimeError,
) -> PurviewAnswer {
    match req {
        PurviewRequest::List(_) => PurviewAnswer::Listed(Err(error)),
        PurviewRequest::Resolve(_) => PurviewAnswer::Resolved(Err(error)),
        PurviewRequest::Configure(_) | PurviewRequest::Rig(_) => PurviewAnswer::Rows(Err(error)),
    }
}

/// `work` run on a task of its own, a panic answering as an internal error.
async fn contained<T: Send + 'static>(
    work: impl Future<Output = EngineResult<T>> + Send + 'static,
) -> EngineResult<T> {
    match tk::spawn(work).await {
        Ok(result) => result,
        Err(e) => Err(EngineError::Internal {
            phase: "engine".to_string(),
            reason: format!("the request's task failed: {e}"),
        }),
    }
}

/// The engine's asks of its controller, made from an eval's thread.
#[derive(Clone)]
pub(crate) struct HostLink(subsys::Requester<EngineSystem>);

impl HostLink {
    /// `ask` answered by the engine's controller, or why it went unanswered.
    pub(crate) fn ask(
        &self,
        ask: HostAsk,
    ) -> Result<HostAnswer, String> {
        self.0
            .request_blocking(ask)
            .map_err(|e| format!("the engine's host: {e}"))
    }
}

/// The engine's running system, and the handle its controller requests through.
pub struct EngineHandle {
    control: subsys::SystemControl<EngineSystem>,
}

impl EngineHandle {
    /// Starts the engine's system on `init`'s namespace, `handler` its controller's.
    pub async fn start(handler: impl subsys::Handler<EngineSystem>) -> EngineResult<Self> {
        let control = subsys::SystemControl::start(
            EngineSysPaths::of(started()),
            EngineSysConfig,
            EngineSysParams,
            handler,
        )
        .await
        .map_err(|e| EngineError::Init {
            reason: format!("grammar: the engine's system: {e}"),
        })?;
        Ok(Self { control })
    }

    /// Sends a request and awaits its answer.
    pub async fn request<T>(&self, req: T) -> EngineResult<T::ResponseType>
    where
        T: subsys::Request<EngineSystem>,
        T::ResponseType: TryFrom<FromEngine, Error = subsys::SubsysError>,
    {
        self.control.request(req).await.map_err(|e| EngineError::Internal {
            phase: "engine::request".to_string(),
            reason: e.to_string(),
        })
    }

    /// Sends a command to the local engine and awaits its answer.
    pub async fn control(&self, command: EngineCommand) -> EngineResult<ControlAnswer> {
        self.request(ControlRequest {
            host: Host::Local,
            command,
        })
        .await?
    }

    /// Stops the engine's system, its calls in flight torn down.
    pub async fn stop(self) -> EngineResult<()> {
        self.control.stop(true).await.map_err(|e| EngineError::Internal {
            phase: "engine::stop".to_string(),
            reason: e.to_string(),
        })
    }
}
