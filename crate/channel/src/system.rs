use crate::*;

/// The Channel as a subsys system: its state, its gate, and its sends.
pub struct ChannelSystem {
    inner: subsys::InnerSystem<Self>,
    channel: Channel,
}

/// The system's parameters: the spam thresholds it starts on.
#[cereal::derived(Data, Copy, Eq)]
pub struct ChannelSysParams {
    pub spam: SpamThresholdsData,
}

impl subsys::Params for ChannelSysParams {}

/// The system's configuration, which holds nothing.
#[derive(Default, Debug, Clone, ser::Serialize, ser::Deserialize)]
pub struct ChannelSysConfig;

impl subsys::Config for ChannelSysConfig {}

/// The namespace's homes as the system's paths, which it never writes.
#[derive(Debug, Clone)]
pub struct ChannelSysPaths {
    data_home: PathBuf,
    cache_home: PathBuf,
}

impl ChannelSysPaths {
    /// The paths of a namespace with these data and cache homes.
    pub fn new(
        data_home: PathBuf,
        cache_home: PathBuf,
    ) -> Self {
        Self {
            data_home,
            cache_home,
        }
    }

    /// The namespace's secrets, its keypair's directory.
    fn secrets_dir(&self) -> PathBuf {
        self.data_home.join("keypair")
    }
}

impl agnostic::AppPaths for ChannelSysPaths {
    fn cache_dir(&self) -> PathBuf {
        self.cache_home.clone()
    }

    fn config_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn config_secrets_dir(&self) -> PathBuf {
        self.secrets_dir()
    }

    fn data_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn state_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn state_secrets_dir(&self) -> PathBuf {
        self.secrets_dir()
    }

    fn runtime_dir(&self) -> PathBuf {
        self.cache_home.clone()
    }
}

impl agnostic::HasAppPaths for ChannelSysPaths {
    fn app_paths(&self) -> &impl agnostic::AppPaths {
        self
    }
}

impl ChannelSystem {
    /// A message's answer, none for a notice.
    async fn answer(
        &mut self,
        msg: ToChannelSys,
    ) -> Option<FromChannelSys> {
        Some(match msg {
            ToChannelSys::Open(req) => {
                self.channel.open(req);
                FromChannelSys::Opened(Opened)
            }
            ToChannelSys::Close(req) => FromChannelSys::Closed(self.channel.close_request(req)),
            ToChannelSys::Expire(req) => FromChannelSys::Closed(Closed {
                closed: self.channel.close_if_unverified(req.epoch, req.code, &req.reason),
                inbox: None,
            }),
            ToChannelSys::Hub(notice) => {
                self.channel.hub_notice(notice);
                return None;
            }
            ToChannelSys::Ready(ReadyRequest) => FromChannelSys::Ready(self.channel.mark_verified()),
            ToChannelSys::Sync(SyncRequest) => FromChannelSys::Synced(self.channel.take_sync()),
            ToChannelSys::Status(StatusRequest) => FromChannelSys::Status(self.channel.status()),
            ToChannelSys::Thresholds(update) => FromChannelSys::Thresholds(
                self.channel.set_thresholds(update).map(SpamThresholds::data),
            ),
            ToChannelSys::Host(HostRequest { line }) => {
                FromChannelSys::Hosted(self.channel.send_control(line))
            }
            ToChannelSys::Emit(req) => FromChannelSys::Emitted(self.channel.emit_packet(req)),
            ToChannelSys::Send(SendRequest(data)) => FromChannelSys::Sent(self.send(data).await),
            ToChannelSys::Stopped(StoppedNotice(stopped)) => {
                self.channel.stopped(stopped);
                return None;
            }
        })
    }

    /// A body's send: its checks, its files written off the loop, then its emit.
    async fn send(
        &mut self,
        data: ChannelSendData,
    ) -> SendVerdict {
        let mut planned = match self.channel.plan_send(data, Instant::now()) {
            Ok(planned) => planned,
            Err(verdict) => return verdict,
        };
        let files = std::mem::take(&mut planned.files);
        if !files.is_empty() {
            match tk::spawn_blocking(move || write_files(files)).await {
                Ok(Ok(())) => {}
                Ok(Err(reason)) => return SendVerdict::Refused(reason),
                Err(e) => return SendVerdict::Refused(format!("the inbox write failed: {e}")),
            }
        }
        self.channel.finish_send(planned)
    }

    /// What the step sent up, each a singular packet to the controller.
    async fn send_ups(&mut self) {
        for up in self.channel.take_ups() {
            let msg = match up.kind {
                UpKind::Line(line) => FromChannelSys::Line(Line {
                    epoch: up.epoch,
                    line,
                }),
                UpKind::CloseHub { code, reason } => FromChannelSys::CloseHub(CloseHub {
                    epoch: up.epoch,
                    code,
                    reason,
                }),
                UpKind::Emergency(emergency) => FromChannelSys::Emergency(emergency),
            };
            let packet = subsys::Packet::singular(msg);
            let _ = self.inner.channel.tx.send(subsys::MsgFromSys::Packet(packet)).await;
        }
    }
}

impl subsys::System for ChannelSystem {
    const CHANNEL_SIZE: usize = 100;

    type Paths = ChannelSysPaths;
    type Params = ChannelSysParams;
    type Config = ChannelSysConfig;
    type ToSys = ToChannelSys;
    type FromSys = FromChannelSys;
    type Flow = subsys::StdFlow;

    fn inner(&self) -> &subsys::InnerSystem<Self> { &self.inner }
    fn inner_mut(&mut self) -> &mut subsys::InnerSystem<Self> { &mut self.inner }

    async fn init(inner: subsys::InnerSystem<Self>, params: Self::Params) -> subsys::SubsysResult<Self> {
        let thresholds = SpamThresholds::from_data(params.spam).map_err(|e| {
            eprintln!("grammar: the Channel's thresholds: {e}");
            subsys::SubsysError::Fatal
        })?;
        Ok(Self {
            inner,
            channel: Channel::new(thresholds),
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

    async fn on_channel_recv(&mut self, pkt: subsys::Packet<ToChannelSys>) -> subsys::FlowResult<Self::Flow> {
        let (id, _nature, msg) = pkt.into_tuple();
        let answer = self.answer(msg).await;
        self.send_ups().await;
        if let Some(answer) = answer {
            let response = subsys::Packet::response(id, answer);
            let _ = self.inner.channel.tx.send(subsys::MsgFromSys::Packet(response)).await;
        }
        Ok(subsys::Flow::Continue)
    }

    async fn on_stop(&mut self, _halt: bool) -> subsys::RunResult {
        subsys::SUCCESS
    }

    async fn on_resume(&mut self) -> subsys::SysResult<bool> {
        Ok(true)
    }
}

/// The Channel's running system, and the handle its face and senders ask through.
pub struct ChannelHandle {
    control: subsys::SystemControl<ChannelSystem>,
    /// The runtime the system runs on, which a thread outside it blocks on.
    runtime: tk::Handle,
}

impl ChannelHandle {
    /// Starts the Channel's system, `handler` taking what it sends up.
    pub async fn start(
        paths: ChannelSysPaths,
        params: ChannelSysParams,
        handler: impl subsys::Handler<ChannelSystem>,
    ) -> Result<Self, String> {
        let control = subsys::SystemControl::start(paths, ChannelSysConfig, params, handler)
            .await
            .map_err(system_error)?;
        Ok(Self {
            control,
            runtime: tk::Handle::current(),
        })
    }

    /// Sends a request and awaits its answer.
    pub async fn request<T>(
        &self,
        req: T,
    ) -> Result<T::ResponseType, String>
    where
        T: subsys::Request<ChannelSystem>,
        T::ResponseType: TryFrom<FromChannelSys, Error = subsys::SubsysError>,
    {
        self.control.request(req).await.map_err(system_error)
    }

    /// A request from a thread outside the runtime, its answer awaited.
    pub fn request_blocking<T>(
        &self,
        req: T,
    ) -> Result<T::ResponseType, String>
    where
        T: subsys::Request<ChannelSystem>,
        T::ResponseType: TryFrom<FromChannelSys, Error = subsys::SubsysError>,
    {
        self.runtime.block_on(self.request(req))
    }

    /// Sends a notice, which the system takes without an answer.
    pub async fn notify(
        &self,
        notice: impl Into<ToChannelSys>,
    ) -> Result<(), String> {
        let packet = subsys::Packet::singular(notice.into());
        self.control.send_packet(packet).await.map_err(system_error)
    }

    /// A notice from a thread outside the runtime.
    pub fn notify_blocking(
        &self,
        notice: impl Into<ToChannelSys>,
    ) -> Result<(), String> {
        self.runtime.block_on(self.notify(notice))
    }

    /// Stops the Channel's system.
    pub async fn stop(self) -> Result<(), String> {
        self.control.stop(true).await.map_err(system_error)
    }
}

/// The system's failure as the Channel's callers report it.
fn system_error(e: subsys::SubsysError) -> String {
    format!("grammar: the Channel's system: {e}")
}
