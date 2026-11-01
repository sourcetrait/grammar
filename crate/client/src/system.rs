//! Remote engines as a subsys system, a session a host opened at its first request.
use crate::*;

/// How long a stop waits for its requests to let the system go.
const RELEASE_GRACE: Duration = Duration::from_secs(1);

/// What the controller sends: a request, the close, or an ask's answer.
#[cereal::derived(Data)]
pub enum ToRemote {
    Request(RemoteRequest),
    /// Every session closed, its requests answered `daemon::session_lost`.
    Close,
    Answer(HostAnswer),
}

/// What the system sends up: an answer, a worker's ask, or its emergency.
#[cereal::derived(Data)]
pub enum FromRemote {
    Response(RemoteResponse),
    Closed,
    Ask(RemoteAsk),
    Emergency(RemoteEmergency),
}

/// A request of the engine an alias names.
#[cereal::derived(Data)]
pub struct RemoteRequest {
    pub alias: String,
    pub request: EngineRequest,
}

/// A request's response from its host's worker, or why there is none.
#[cereal::derived(Data)]
pub struct RemoteResponse(pub Result<EngineResponse, TransportError>);

/// A worker's ask of the MCP, under the alias of its engine.
#[cereal::derived(Data)]
pub struct RemoteAsk {
    pub alias: String,
    pub ask: SessionAsk,
}

/// A worker's emergency, under the alias of its engine.
#[cereal::derived(Data)]
pub struct RemoteEmergency {
    pub alias: String,
    pub emergency: Emergency,
}

impl subsys::Request<RemoteSystem> for RemoteRequest {
    type ResponseType = RemoteResponse;
}

impl From<RemoteRequest> for ToRemote {
    fn from(v: RemoteRequest) -> Self {
        Self::Request(v)
    }
}

impl TryFrom<FromRemote> for RemoteResponse {
    type Error = subsys::SubsysError;

    fn try_from(v: FromRemote) -> subsys::SubsysResult<Self> {
        match v {
            FromRemote::Response(response) => Ok(response),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

/// The close's request, answered once every session has closed.
#[cereal::derived(Data, Copy, Eq)]
pub struct CloseRequest;

impl subsys::Request<RemoteSystem> for CloseRequest {
    type ResponseType = Closed;
}

impl From<CloseRequest> for ToRemote {
    fn from(_: CloseRequest) -> Self {
        Self::Close
    }
}

/// The close's answer.
#[cereal::derived(Data, Copy, Eq)]
pub struct Closed;

impl TryFrom<FromRemote> for Closed {
    type Error = subsys::SubsysError;

    fn try_from(v: FromRemote) -> subsys::SubsysResult<Self> {
        match v {
            FromRemote::Closed => Ok(Closed),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl subsys::Ask<RemoteSystem> for RemoteAsk {
    type Answer = HostAnswer;
}

impl From<RemoteAsk> for FromRemote {
    fn from(v: RemoteAsk) -> Self {
        Self::Ask(v)
    }
}

impl TryFrom<ToRemote> for HostAnswer {
    type Error = subsys::SubsysError;

    fn try_from(v: ToRemote) -> subsys::SubsysResult<Self> {
        match v {
            ToRemote::Answer(answer) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

/// The system's parameters: the engines, and the bounds each session asks.
#[cereal::derived(Data, Eq)]
pub struct RemoteSysParams {
    pub endpoints: Vec<Endpoint>,
    pub limits: Limits,
}

impl subsys::Params for RemoteSysParams {}

/// The system's configuration, which holds nothing.
#[derive(Default, Debug, Clone, ser::Serialize, ser::Deserialize)]
pub struct RemoteSysConfig;

impl subsys::Config for RemoteSysConfig {}

/// The system's paths, the MCP's homes, of which it reads nothing.
#[derive(Debug, Clone)]
pub struct RemoteSysPaths {
    data_home: PathBuf,
    cache_home: PathBuf,
}

impl RemoteSysPaths {
    /// The paths of the MCP's data and cache homes.
    pub fn new(data_home: PathBuf, cache_home: PathBuf) -> Self {
        Self { data_home, cache_home }
    }
}

impl agnostic::AppPaths for RemoteSysPaths {
    fn cache_dir(&self) -> PathBuf {
        self.cache_home.clone()
    }

    fn config_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn config_secrets_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn data_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn state_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn state_secrets_dir(&self) -> PathBuf {
        self.data_home.clone()
    }

    fn runtime_dir(&self) -> PathBuf {
        self.cache_home.clone()
    }
}

impl agnostic::HasAppPaths for RemoteSysPaths {
    fn app_paths(&self) -> &impl agnostic::AppPaths {
        self
    }
}

/// The remote engines' system, each request answered from a task of its own.
pub struct RemoteSystem {
    inner: subsys::InnerSystem<Self>,
    hosts: Arc<Hosts>,
}

impl subsys::System for RemoteSystem {
    const CHANNEL_SIZE: usize = 100;

    type Paths = RemoteSysPaths;
    type Params = RemoteSysParams;
    type Config = RemoteSysConfig;
    type ToSys = ToRemote;
    type FromSys = FromRemote;
    type Flow = subsys::StdFlow;

    fn inner(&self) -> &subsys::InnerSystem<Self> {
        &self.inner
    }

    fn inner_mut(&mut self) -> &mut subsys::InnerSystem<Self> {
        &mut self.inner
    }

    async fn init(inner: subsys::InnerSystem<Self>, params: Self::Params) -> subsys::SubsysResult<Self> {
        let uplink = Uplink {
            requester: inner.requester(),
            tx: inner.channel.tx.clone(),
        };
        let hosts = Arc::new(Hosts::new(params, uplink));
        Ok(Self { inner, hosts })
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

    async fn on_channel_recv(&mut self, pkt: subsys::Packet<ToRemote>) -> subsys::FlowResult<Self::Flow> {
        let (id, _nature, msg) = pkt.into_tuple();
        let hosts = self.hosts.clone();
        let tx = self.inner.channel.tx.clone();
        match msg {
            ToRemote::Request(RemoteRequest { alias, request }) => {
                tk::spawn(async move {
                    let work = tk::spawn(async move { hosts.request(&alias, request).await });
                    let response = work.await.unwrap_or_else(|e| {
                        Err(TransportError::Unavailable {
                            reason: format!("the request's task failed: {e}"),
                        })
                    });
                    respond(&tx, id, FromRemote::Response(RemoteResponse(response))).await;
                });
            }
            ToRemote::Close => {
                tk::spawn(async move {
                    let _ = tk::spawn(async move { hosts.close_all().await }).await;
                    respond(&tx, id, FromRemote::Closed).await;
                });
            }
            ToRemote::Answer(_) => {}
        }
        Ok(subsys::Flow::Continue)
    }

    async fn on_stop(&mut self, _halt: bool) -> subsys::RunResult {
        self.hosts.close_all().await;
        subsys::SUCCESS
    }

    async fn on_resume(&mut self) -> subsys::SysResult<bool> {
        Ok(true)
    }
}

/// `answer` sent up as the response to request `id`.
async fn respond(tx: &tk::Sender<subsys::MsgFromSys<FromRemote>>, id: subsys::MsgID, answer: FromRemote) {
    let _ = tx.send(subsys::MsgFromSys::Packet(subsys::Packet::response(id, answer))).await;
}

/// The system's way up: its asks awaited, its emergencies sent as packets.
#[derive(Clone)]
struct Uplink {
    requester: subsys::Requester<RemoteSystem>,
    tx: tk::Sender<subsys::MsgFromSys<FromRemote>>,
}

impl Uplink {
    /// A worker's ask answered by the controller, or refused saying why it wasn't.
    async fn ask(&self, ask: RemoteAsk) -> HostAnswer {
        match self.requester.request(ask).await {
            Ok(answer) => answer,
            Err(e) => HostAnswer::Refused(format!("the MCP left the ask unanswered: {e}")),
        }
    }

    /// A worker's emergency sent up as a packet no request waits on.
    async fn emergency(&self, emergency: RemoteEmergency) {
        let packet = subsys::Packet::singular(FromRemote::Emergency(emergency));
        let _ = self.tx.send(subsys::MsgFromSys::Packet(packet)).await;
    }
}

/// A host's session opening, its outcome shared by each request waiting on it.
type Opening = Shared<BoxFuture<'static, Result<Arc<RemoteSession>, TransportError>>>;

/// A host's session: being opened, or open.
enum Slot {
    Opening(Opening),
    Open(Arc<RemoteSession>),
}

/// The configured engines and each one's session.
struct Hosts {
    endpoints: HashMap<String, Endpoint>,
    limits: Limits,
    slots: std::sync::Mutex<HashMap<String, Slot>>,
    uplink: Uplink,
    /// Set once the close begins, after which no session opens.
    closing: AtomicBool,
}

impl Hosts {
    /// The engines by alias, none of their sessions open.
    fn new(params: RemoteSysParams, uplink: Uplink) -> Self {
        Self {
            endpoints: params.endpoints.into_iter().map(|endpoint| (endpoint.alias.clone(), endpoint)).collect(),
            limits: params.limits,
            slots: std::sync::Mutex::new(HashMap::new()),
            uplink,
            closing: AtomicBool::new(false),
        }
    }

    /// The slots, a poisoned lock taken as it stands.
    fn slots(&self) -> std::sync::MutexGuard<'_, HashMap<String, Slot>> {
        self.slots.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// `request`'s response from `alias`'s worker, a session opened where none is.
    async fn request(&self, alias: &str, request: EngineRequest) -> Result<EngineResponse, TransportError> {
        self.session(alias).await?.request(request).await
    }

    /// `alias`'s open session, or one opened, requests arriving meanwhile sharing it.
    async fn session(&self, alias: &str) -> Result<Arc<RemoteSession>, TransportError> {
        let opening = {
            let mut slots = self.slots();
            if self.closing.load(Ordering::SeqCst) {
                return Err(closing());
            }
            match slots.get(alias) {
                Some(Slot::Open(session)) if !session.has_ended() => return Ok(session.clone()),
                Some(Slot::Opening(opening)) => opening.clone(),
                _ => {
                    let Some(endpoint) = self.endpoints.get(alias).cloned() else {
                        return Err(TransportError::Unavailable {
                            reason: format!("no engine is configured as `{alias}`"),
                        });
                    };
                    let opening = open_relayed(endpoint, self.limits, self.uplink.clone()).boxed().shared();
                    let ended = slots.insert(alias.to_string(), Slot::Opening(opening.clone()));
                    if let Some(Slot::Open(ended)) = ended {
                        tk::spawn(async move { ended.close().await });
                    }
                    opening
                }
            }
        };
        let opened = opening.clone().await;
        let stray = {
            let mut slots = self.slots();
            let current = matches!(slots.get(alias), Some(Slot::Opening(slot)) if slot.ptr_eq(&opening));
            if current {
                slots.remove(alias);
            }
            match &opened {
                Ok(session) if self.closing.load(Ordering::SeqCst) => Some(session.clone()),
                Ok(session) if current => {
                    slots.insert(alias.to_string(), Slot::Open(session.clone()));
                    None
                }
                _ => None,
            }
        };
        if let Some(session) = stray {
            session.close().await;
            return Err(closing());
        }
        opened
    }

    /// Every session closed, none opening after, each request in flight let go.
    async fn close_all(&self) {
        self.closing.store(true, Ordering::SeqCst);
        let sessions: Vec<Arc<RemoteSession>> = self
            .slots()
            .drain()
            .filter_map(|(_, slot)| match slot {
                Slot::Open(session) => Some(session),
                Slot::Opening(_) => None,
            })
            .collect();
        join_all(sessions.iter().map(|session| session.close())).await;
    }
}

/// The refusal a request meets once the sessions are closing.
fn closing() -> TransportError {
    TransportError::SessionLost {
        reason: "the MCP is closing its sessions".to_string(),
    }
}

/// `endpoint`'s session opened, its events relayed up for its life.
async fn open_relayed(
    endpoint: Endpoint,
    limits: Limits,
    uplink: Uplink,
) -> Result<Arc<RemoteSession>, TransportError> {
    let (session, events) = open(&endpoint, limits).await.inspect_err(|e| {
        eprintln!("grammar: {}: no session: {}", endpoint.alias, e.message());
    })?;
    let worker = session.worker();
    eprintln!(
        "grammar: {}: a session with {} as pid {}, its nom {}",
        endpoint.alias, worker.instance, worker.pid, worker.nom
    );
    tk::spawn(relay(endpoint.alias, events, uplink));
    Ok(Arc::new(session))
}

/// Events relayed up: each ask answered on a task of its own, each emergency sent.
async fn relay(alias: String, mut events: tk::UnboundedReceiver<SessionEvent>, uplink: Uplink) {
    while let Some(event) = events.recv().await {
        match event {
            SessionEvent::Ask { ask, responder } => {
                let uplink = uplink.clone();
                let alias = alias.clone();
                tk::spawn(async move {
                    responder.answer(uplink.ask(RemoteAsk { alias, ask }).await);
                });
            }
            SessionEvent::Emergency(emergency) => {
                uplink.emergency(RemoteEmergency { alias: alias.clone(), emergency }).await;
            }
            SessionEvent::Ended(reason) => {
                eprintln!("grammar: {alias}: the session ended: {}", reason.message());
            }
        }
    }
}

/// The remote engines' system, and the handle its controller requests through.
pub struct RemoteHandle {
    control: subsys::SystemControl<RemoteSystem>,
}

impl RemoteHandle {
    /// Starts the system on `params`' engines, `handler` its controller's.
    pub async fn start(
        paths: RemoteSysPaths,
        params: RemoteSysParams,
        handler: impl subsys::Handler<RemoteSystem>,
    ) -> Result<Self, String> {
        let control = subsys::SystemControl::start(paths, RemoteSysConfig, params, handler)
            .await
            .map_err(|e| format!("grammar: the remote engines' system: {e}"))?;
        Ok(Self { control })
    }

    /// `request`'s response from `alias`'s worker, or why there is none.
    pub async fn request(&self, alias: &str, request: EngineRequest) -> Result<EngineResponse, TransportError> {
        let request = RemoteRequest {
            alias: alias.to_string(),
            request,
        };
        match self.control.request(request).await {
            Ok(RemoteResponse(response)) => response,
            Err(e) => Err(TransportError::Unavailable {
                reason: format!("the remote engines' system: {e}"),
            }),
        }
    }

    /// Every session closed, the requests in flight answered `daemon::session_lost`.
    pub async fn close(&self) {
        let _ = self.control.request(CloseRequest).await;
    }

    /// The system stopped once its requests let it go, within a second.
    pub async fn stop(handle: Arc<Self>) -> Result<(), String> {
        let deadline = tokio::time::Instant::now() + RELEASE_GRACE;
        let mut handle = handle;
        loop {
            match Arc::try_unwrap(handle) {
                Ok(handle) => {
                    return handle
                        .control
                        .stop(true)
                        .await
                        .map_err(|e| format!("grammar: the remote engines' stop: {e}"));
                }
                Err(shared) if tokio::time::Instant::now() < deadline => {
                    handle = shared;
                    tk::sleep(Duration::from_millis(10)).await;
                }
                Err(_) => return Err("grammar: the remote engines' system is still held".to_string()),
            }
        }
    }
}
