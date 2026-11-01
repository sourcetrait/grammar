use crate::*;

/// The host's own key pair, which every link presents.
#[cereal::derived(Data, Eq)]
pub struct IdentityKeys {
    pub cert: String,
    pub key: String,
}

/// A variable's stand-in when it is unset, for expanding a certificate's path.
#[cereal::derived(Data, Eq)]
pub struct ExpandDefault {
    pub var: String,
    pub value: String,
}

/// `interface remote channel list`: every remote, configured or live.
#[cereal::derived(Data, Copy, Eq)]
pub struct RemoteListRequest;

/// `interface remote channel get`: one remote's entry and state.
#[cereal::derived(Data, Eq)]
pub struct RemoteGetRequest {
    pub alias: String,
}

/// `interface remote channel set`: make or change an entry.
#[cereal::derived(Data, Eq)]
pub struct RemoteSetRequest {
    pub alias: String,
    pub address: Option<String>,
    pub listen: Option<String>,
    pub cert: Option<String>,
}

/// `interface remote channel unset`: remove an entry; a live link stays.
#[cereal::derived(Data, Eq)]
pub struct RemoteUnsetRequest {
    pub alias: String,
}

/// `interface remote channel listen`: bind for a peer, answering the endpoint.
#[cereal::derived(Data, Eq)]
pub struct RemoteListenRequest {
    pub alias: String,
    /// The `ip` or `ip:port` to bind, in place of the entry's.
    pub on: Option<String>,
}

/// `interface remote channel connect`: dial a peer, answering once both connect.
#[cereal::derived(Data, Eq)]
pub struct RemoteConnectRequest {
    pub alias: String,
    /// The `ip:port` to dial, in place of the entry's.
    pub address: Option<String>,
}

/// `interface remote channel close`: close an open link or a waiting listener.
#[cereal::derived(Data, Eq)]
pub struct RemoteCloseRequest {
    pub alias: String,
}

/// One file of a send: the name its receiver lands it under, and its bytes.
#[cereal::derived(Data, Eq)]
pub struct Payload {
    pub dest: String,
    pub bytes: Vec<u8>,
}

/// A body's send to the peer of `mcp_nom`, its event rendered and its files read.
#[cereal::derived(Data, Eq)]
pub struct RemoteSendRequest {
    pub mcp_nom: String,
    pub model: String,
    pub event_nuon: String,
    pub payloads: Vec<Payload>,
}

/// The Channel's inbox, set at each open and cleared when a close takes it.
#[cereal::derived(Data, Eq)]
pub struct InboxRequest(pub Option<String>);

/// A peer's message for the Channel, and whether its files landed beside it.
#[cereal::derived(Data)]
pub struct Relay {
    pub packet: RemotePacket,
    pub files: bool,
}

impl Relay {
    /// The inbox-relative directory its files landed in, if any.
    pub fn attached(&self) -> Option<String> {
        self.files
            .then(|| format!("{}/{}", self.packet.origin, self.packet.delivery))
    }
}

/// What the peer links' controller asks of them.
#[cereal::derived(Data)]
pub enum ToPeerSys {
    List(RemoteListRequest),
    Get(RemoteGetRequest),
    Set(RemoteSetRequest),
    Unset(RemoteUnsetRequest),
    Listen(RemoteListenRequest),
    Connect(RemoteConnectRequest),
    Close(RemoteCloseRequest),
    Send(RemoteSendRequest),
    Inbox(InboxRequest),
}

/// Each request's answer, and what the links send up unasked.
#[cereal::derived(Data)]
pub enum FromPeerSys {
    Remote(Result<RemoteResult, RuntimeError>),
    Sent(Result<String, String>),
    /// The inbox the links now hold.
    Inbox(Option<String>),
    Relay(Relay),
    Report(RemoteChannelEvent),
}

/// A message's way into `ToPeerSys`.
macro_rules! into_to_sys {
    ($($ty:ty => $variant:ident),* $(,)?) => {$(
        impl From<$ty> for ToPeerSys {
            fn from(v: $ty) -> Self { Self::$variant(v) }
        }
    )*};
}

into_to_sys! {
    RemoteListRequest => List,
    RemoteGetRequest => Get,
    RemoteSetRequest => Set,
    RemoteUnsetRequest => Unset,
    RemoteListenRequest => Listen,
    RemoteConnectRequest => Connect,
    RemoteCloseRequest => Close,
    RemoteSendRequest => Send,
    InboxRequest => Inbox,
}

impl TryFrom<FromPeerSys> for Result<RemoteResult, RuntimeError> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromPeerSys) -> subsys::SubsysResult<Self> {
        match v {
            FromPeerSys::Remote(answer) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromPeerSys> for Result<String, String> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromPeerSys) -> subsys::SubsysResult<Self> {
        match v {
            FromPeerSys::Sent(answer) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromPeerSys> for Option<String> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromPeerSys) -> subsys::SubsysResult<Self> {
        match v {
            FromPeerSys::Inbox(held) => Ok(held),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

/// Each remote command's request, answering its result or its refusal.
macro_rules! remote_request {
    ($($ty:ty),* $(,)?) => {$(
        impl subsys::Request<PeerSystem> for $ty {
            type ResponseType = Result<RemoteResult, RuntimeError>;
        }
    )*};
}

remote_request! {
    RemoteListRequest,
    RemoteGetRequest,
    RemoteSetRequest,
    RemoteUnsetRequest,
    RemoteListenRequest,
    RemoteConnectRequest,
    RemoteCloseRequest,
}

impl subsys::Request<PeerSystem> for RemoteSendRequest {
    type ResponseType = Result<String, String>;
}

impl subsys::Request<PeerSystem> for InboxRequest {
    type ResponseType = Option<String>;
}
