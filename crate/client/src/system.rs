//! Remote engines as a subsys system, a session a host opened at its first request.
use crate::*;

/// How long a stop waits for its requests to let the system go.
const RELEASE_GRACE: Duration = Duration::from_secs(1);

/// What the controller sends: a request, a session command, or an ask's answer.
#[cereal::derived(Data)]
pub enum ToRemote {
    Request(RemoteRequest),
    /// Every session closed, its requests answered `remote::engine::session_lost`.
    Close,
    Answer(HostAnswer),
    States,
    Connect(Endpoint),
    CloseSession(String),
}

/// What the system sends up: an answer, a worker's ask, or its emergency.
#[cereal::derived(Data)]
pub enum FromRemote {
    Response(RemoteResponse),
    Closed,
    Ask(RemoteAsk),
    Emergency(RemoteEmergency),
    States(SessionStates),
    Connected(ConnectAnswer),
    SessionClosed(SessionClosed),
}

/// A request of the engine an entry names, its session opened on that entry.
#[cereal::derived(Data)]
pub struct RemoteRequest {
    pub endpoint: Endpoint,
    pub request: EngineRequest,
}

/// A session's phase: none, opening, or open.
#[cereal::derived(Data, Copy, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionPhase {
    Closed,
    Opening,
    Open,
}

/// One alias's session: its phase, its worker while open, its last open's failure.
#[cereal::derived(Data, Eq)]
pub struct SessionState {
    pub alias: String,
    pub phase: SessionPhase,
    pub worker_nom: Option<String>,
    pub worker_pid: Option<u32>,
    pub failure: Option<String>,
}

/// The states request: every alias with a session or a failure to report.
#[cereal::derived(Data, Copy, Eq)]
pub struct StatesRequest;

/// The states request's answer, by alias.
#[cereal::derived(Data, Eq)]
pub struct SessionStates(pub Vec<SessionState>);

/// The connect request: a session opened on `endpoint` now, if none is open.
#[cereal::derived(Data, Eq)]
pub struct ConnectRequest {
    pub endpoint: Endpoint,
}

/// The connect's answer: the session's state, or why it didn't open.
#[cereal::derived(Data)]
pub struct ConnectAnswer(pub Result<SessionState, TransportError>);

/// The close of one alias's session.
#[cereal::derived(Data, Eq)]
pub struct CloseSessionRequest {
    pub alias: String,
}

/// Whether the closed session was open.
#[cereal::derived(Data, Copy, Eq)]
pub struct SessionClosed(pub bool);

impl subsys::Request<RemoteSystem> for StatesRequest {
    type ResponseType = SessionStates;
}

impl From<StatesRequest> for ToRemote {
    fn from(_: StatesRequest) -> Self {
        Self::States
    }
}

impl TryFrom<FromRemote> for SessionStates {
    type Error = subsys::SubsysError;

    fn try_from(v: FromRemote) -> subsys::SubsysResult<Self> {
        match v {
            FromRemote::States(states) => Ok(states),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl subsys::Request<RemoteSystem> for ConnectRequest {
    type ResponseType = ConnectAnswer;
}

impl From<ConnectRequest> for ToRemote {
    fn from(v: ConnectRequest) -> Self {
        Self::Connect(v.endpoint)
    }
}

impl TryFrom<FromRemote> for ConnectAnswer {
    type Error = subsys::SubsysError;

    fn try_from(v: FromRemote) -> subsys::SubsysResult<Self> {
        match v {
            FromRemote::Connected(answer) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl subsys::Request<RemoteSystem> for CloseSessionRequest {
    type ResponseType = SessionClosed;
}

impl From<CloseSessionRequest> for ToRemote {
    fn from(v: CloseSessionRequest) -> Self {
        Self::CloseSession(v.alias)
    }
}

impl TryFrom<FromRemote> for SessionClosed {
    type Error = subsys::SubsysError;

    fn try_from(v: FromRemote) -> subsys::SubsysResult<Self> {
        match v {
            FromRemote::SessionClosed(closed) => Ok(closed),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
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

/// The system's parameters: each session's bounds; a request brings its entry.
#[cereal::derived(Data, Eq)]
pub struct RemoteSysParams {
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
            ToRemote::Request(RemoteRequest { endpoint, request }) => {
                tk::spawn(async move {
                    let work = tk::spawn(async move { hosts.request(&endpoint, request).await });
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
            ToRemote::States => {
                respond(&tx, id, FromRemote::States(SessionStates(hosts.states()))).await;
            }
            ToRemote::Connect(endpoint) => {
                tk::spawn(async move {
                    let work = tk::spawn(async move { hosts.connect(&endpoint).await });
                    let answer = work.await.unwrap_or_else(|e| {
                        Err(TransportError::Unavailable {
                            reason: format!("the connect's task failed: {e}"),
                        })
                    });
                    respond(&tx, id, FromRemote::Connected(ConnectAnswer(answer))).await;
                });
            }
            ToRemote::CloseSession(alias) => {
                tk::spawn(async move {
                    let work = tk::spawn(async move { hosts.close_session(&alias).await });
                    let was_open = work.await.unwrap_or(false);
                    respond(&tx, id, FromRemote::SessionClosed(SessionClosed(was_open))).await;
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

/// Each alias's session and each alias's last failure to open one.
struct Hosts {
    limits: Limits,
    slots: std::sync::Mutex<HashMap<String, Slot>>,
    /// The last open's failure by alias, cleared when an open succeeds.
    failures: std::sync::Mutex<HashMap<String, String>>,
    uplink: Uplink,
    /// Set once the close begins, after which no session opens.
    closing: AtomicBool,
}

impl Hosts {
    /// No session open, each to open on the entry its first request brings.
    fn new(params: RemoteSysParams, uplink: Uplink) -> Self {
        Self {
            limits: params.limits,
            slots: std::sync::Mutex::new(HashMap::new()),
            failures: std::sync::Mutex::new(HashMap::new()),
            uplink,
            closing: AtomicBool::new(false),
        }
    }

    /// The slots, a poisoned lock taken as it stands.
    fn slots(&self) -> std::sync::MutexGuard<'_, HashMap<String, Slot>> {
        self.slots.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The failures, a poisoned lock taken as it stands.
    fn failures(&self) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
        self.failures.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// `request`'s response from `endpoint`'s worker, a session opened where none is.
    async fn request(&self, endpoint: &Endpoint, request: EngineRequest) -> Result<EngineResponse, TransportError> {
        self.session(endpoint).await?.request(request).await
    }

    /// `endpoint`'s session opened now where none is open, and its state.
    async fn connect(&self, endpoint: &Endpoint) -> Result<SessionState, TransportError> {
        let session = self.session(endpoint).await?;
        Ok(open_state(&endpoint.alias, &session, None))
    }

    /// The alias's session, open or opening, closed; whether one was open.
    async fn close_session(&self, alias: &str) -> bool {
        let slot = self.slots().remove(alias);
        match slot {
            Some(Slot::Open(session)) => {
                let live = !session.has_ended();
                session.close().await;
                live
            }
            Some(Slot::Opening(opening)) => match opening.await {
                Ok(session) => {
                    session.close().await;
                    true
                }
                Err(_) => false,
            },
            None => false,
        }
    }

    /// Every alias with a session or an open's failure, by alias.
    fn states(&self) -> Vec<SessionState> {
        let slots = self.slots();
        let failures = self.failures();
        let aliases: std::collections::BTreeSet<&String> = slots.keys().chain(failures.keys()).collect();
        aliases
            .into_iter()
            .map(|alias| {
                let failure = failures.get(alias).cloned();
                match slots.get(alias) {
                    Some(Slot::Open(session)) if !session.has_ended() => open_state(alias, session, failure),
                    Some(Slot::Opening(_)) => SessionState {
                        alias: alias.clone(),
                        phase: SessionPhase::Opening,
                        worker_nom: None,
                        worker_pid: None,
                        failure,
                    },
                    _ => SessionState {
                        alias: alias.clone(),
                        phase: SessionPhase::Closed,
                        worker_nom: None,
                        worker_pid: None,
                        failure,
                    },
                }
            })
            .collect()
    }

    /// The entry's open session, or one opened on it that requests meanwhile share.
    async fn session(&self, endpoint: &Endpoint) -> Result<Arc<RemoteSession>, TransportError> {
        let alias = endpoint.alias.as_str();
        let opening = {
            let mut slots = self.slots();
            if self.closing.load(Ordering::SeqCst) {
                return Err(closing());
            }
            match slots.get(alias) {
                Some(Slot::Open(session)) if !session.has_ended() => return Ok(session.clone()),
                Some(Slot::Opening(opening)) => opening.clone(),
                _ => {
                    let opening = open_relayed(endpoint.clone(), self.limits, self.uplink.clone()).boxed().shared();
                    let ended = slots.insert(alias.to_string(), Slot::Opening(opening.clone()));
                    if let Some(Slot::Open(ended)) = ended {
                        tk::spawn(async move { ended.close().await });
                    }
                    opening
                }
            }
        };
        let opened = opening.clone().await;
        match &opened {
            Ok(_) => {
                self.failures().remove(alias);
            }
            Err(e) => {
                self.failures().insert(alias.to_string(), format!("{}: {}", e.kind(), e.message()));
            }
        }
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

/// An open session's state, its worker's nom and pid.
fn open_state(
    alias: &str,
    session: &RemoteSession,
    failure: Option<String>,
) -> SessionState {
    let worker = session.worker();
    SessionState {
        alias: alias.to_string(),
        phase: SessionPhase::Open,
        worker_nom: Some(worker.nom.clone()),
        worker_pid: Some(worker.pid),
        failure,
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
    /// Starts the system, `handler` its controller's; no session opens until asked.
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

    /// `request`'s response from `endpoint`'s worker, or why there is none.
    pub async fn request(&self, endpoint: Endpoint, request: EngineRequest) -> Result<EngineResponse, TransportError> {
        match self.control.request(RemoteRequest { endpoint, request }).await {
            Ok(RemoteResponse(response)) => response,
            Err(e) => Err(TransportError::Unavailable {
                reason: format!("the remote engines' system: {e}"),
            }),
        }
    }

    /// Each alias with a session or an open's failure; none if the system can't answer.
    pub async fn states(&self) -> Vec<SessionState> {
        match self.control.request(StatesRequest).await {
            Ok(SessionStates(states)) => states,
            Err(_) => Vec::new(),
        }
    }

    /// `endpoint`'s session opened now, if none is open, and its state.
    pub async fn connect(&self, endpoint: Endpoint) -> Result<SessionState, TransportError> {
        match self.control.request(ConnectRequest { endpoint }).await {
            Ok(ConnectAnswer(answer)) => answer,
            Err(e) => Err(TransportError::Unavailable {
                reason: format!("the remote engines' system: {e}"),
            }),
        }
    }

    /// The alias's session closed, its worker ending with it; whether one was open.
    pub async fn close_session(&self, alias: &str) -> bool {
        let request = CloseSessionRequest {
            alias: alias.to_string(),
        };
        matches!(self.control.request(request).await, Ok(SessionClosed(true)))
    }

    /// Each session closed; a request in flight answers `remote::engine::session_lost`.
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
