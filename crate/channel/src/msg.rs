use crate::*;

/// Opens the Channel in its mode on its inbox, under the epoch the face minted.
#[cereal::derived(Data)]
pub struct OpenRequest {
    pub epoch: ChannelEpoch,
    pub mode: ChannelMode,
    /// The directory the open made for the Channel's files.
    pub inbox: String,
    /// A sync Channel's greeting, its first queued line; none in async.
    pub greeting: Option<String>,
}

/// The Channel opened.
#[cereal::derived(Data, Copy, Eq)]
pub struct Opened;

/// Closes the Channel, a close frame's code and reason for its hub.
#[cereal::derived(Data)]
pub struct CloseRequest {
    pub code: u16,
    pub reason: String,
    /// Whether the close takes the inbox, for its closer to prune.
    pub prune: bool,
}

/// Closes the Channel of `epoch` only while it is still unverified.
#[cereal::derived(Data)]
pub struct ExpireRequest {
    pub epoch: ChannelEpoch,
    pub code: u16,
    pub reason: String,
}

/// Whether a close closed an open Channel, and the inbox it took.
#[cereal::derived(Data)]
pub struct Closed {
    pub closed: bool,
    pub inbox: Option<String>,
}

/// What the face's hub tells of its Channel: its claim, and its peer's end.
#[cereal::derived(Data, Copy, Eq)]
pub enum HubNotice {
    /// The hub's first claimant won.
    Claimed(ChannelEpoch),
    /// The claimant's connection ended.
    PeerGone(ChannelEpoch),
}

/// Verifies a claimed Channel, `interface channel ready`, answering its epoch.
#[cereal::derived(Data, Copy, Eq)]
pub struct ReadyRequest;

/// Takes a sync Channel's queued lines, the take its claim.
#[cereal::derived(Data, Copy, Eq)]
pub struct SyncRequest;

/// A sync's lines, and how many the cap dropped since the last.
#[cereal::derived(Data)]
pub struct Synced {
    pub lines: Vec<String>,
    pub dropped: u64,
}

/// Reads the Channel's state.
#[cereal::derived(Data, Copy, Eq)]
pub struct StatusRequest;

/// The Channel's state, and its inbox.
#[cereal::derived(Data)]
pub struct ChannelStatus {
    pub state: ChannelState,
    pub inbox: Option<String>,
}

/// A partial update of the spam thresholds; one of no fields reads them.
#[cereal::derived(Data, Copy, Eq)]
#[derive(Default)]
pub struct ThresholdsRequest {
    pub warn_window_secs: Option<u64>,
    pub warn_rate: Option<u32>,
    pub error_window_secs: Option<u64>,
    pub error_rate: Option<u32>,
}

/// A host packet's rendered line, past the verification gate.
#[cereal::derived(Data)]
pub struct HostRequest {
    pub line: String,
}

/// A host packet through the gate, rendered and its id minted by the Channel.
#[cereal::derived(Data)]
pub struct EmitRequest {
    pub from: String,
    pub model: String,
    pub event: NuVal,
    /// The inbox-relative name its files already sit under, if any.
    pub attached: Option<String>,
}

/// A body's send, its eval's stop held back on the sender's side.
#[cereal::derived(Data)]
pub struct SendRequest(pub ChannelSendData);

/// The stop a sender made of a spam offender, for the error's emergency.
#[cereal::derived(Data)]
pub struct StoppedNotice(pub ChannelSpamErrorEmergency);

/// Why an emit was refused.
#[cereal::derived(Data, Copy, Eq)]
pub enum ChannelSendError {
    /// No channel exists.
    NotOpen,
    /// Open but unverified.
    NotVerified,
    /// The hub's peer is gone while the Channel stays open.
    HubGone,
}

impl ChannelSendError {
    /// The refusal as the sender reports it.
    pub fn message(self) -> &'static str {
        match self {
            Self::NotOpen => "the channel is not open; run `interface channel open` first",
            Self::NotVerified => {
                "the channel is not verified; emitting before `interface channel ready` is \
                 forbidden"
            }
            Self::HubGone => "the channel hub is gone",
        }
    }
}

/// Why a verification was refused.
#[cereal::derived(Data, Copy, Eq)]
pub enum ChannelVerifyError {
    /// Nothing to verify.
    NotOpen,
    /// Open, but nothing has claimed it in its mode.
    NotClaimed(ChannelMode),
}

/// Why a sync was refused.
#[cereal::derived(Data, Copy, Eq)]
pub enum ChannelSyncError {
    /// No channel exists.
    NotOpen,
    /// The channel is open in the other mode, named here.
    WrongMode(ChannelMode),
}

/// Why a host packet through the gate was not emitted.
#[cereal::derived(Data)]
pub enum EmitError {
    /// Its event could not be rendered.
    Render(String),
    /// The gate refused it.
    Refused(ChannelSendError),
}

/// What a body's send came to.
#[cereal::derived(Data)]
pub enum SendVerdict {
    /// Emitted, under the packet's id.
    Sent(String),
    /// Refused, and why.
    Refused(String),
    /// Over the hard threshold: refused, and the sender's eval to be stopped.
    Stop(SpamStop),
}

/// A hard threshold's crossing, as its sender reports and acts on it.
#[cereal::derived(Data)]
pub struct SpamStop {
    pub hits: u32,
    pub window_secs: u64,
    pub rate: u32,
    /// Whether this crossing begins an episode, the one an emergency reports.
    pub notify: bool,
    /// The refusal the sender answers with.
    pub reason: String,
}

/// A rendered packet line for the face to deliver, in async mode.
#[cereal::derived(Data)]
pub struct Line {
    pub epoch: ChannelEpoch,
    pub line: String,
}

/// The Channel of `epoch` closed: its hub writes this close frame and ends.
#[cereal::derived(Data)]
pub struct CloseHub {
    pub epoch: ChannelEpoch,
    pub code: u16,
    pub reason: String,
}

/// What the Channel's controller and face ask of it, and its hub tells it.
#[cereal::derived(Data)]
pub enum ToChannelSys {
    Open(OpenRequest),
    Close(CloseRequest),
    Expire(ExpireRequest),
    Hub(HubNotice),
    Ready(ReadyRequest),
    Sync(SyncRequest),
    Status(StatusRequest),
    Thresholds(ThresholdsRequest),
    Host(HostRequest),
    Emit(EmitRequest),
    Send(SendRequest),
    Stopped(StoppedNotice),
}

/// Each request's answer, and what the Channel sends up unasked.
#[cereal::derived(Data)]
pub enum FromChannelSys {
    Opened(Opened),
    Closed(Closed),
    Ready(Result<ChannelEpoch, ChannelVerifyError>),
    Synced(Result<Synced, ChannelSyncError>),
    Status(ChannelStatus),
    Thresholds(Result<SpamThresholdsData, String>),
    Hosted(Result<(), ChannelSendError>),
    Emitted(Result<String, EmitError>),
    Sent(SendVerdict),
    Line(Line),
    CloseHub(CloseHub),
    Emergency(Emergency),
}

/// A message's way into `ToChannelSys`.
macro_rules! into_to_sys {
    ($($ty:ty => $variant:ident),* $(,)?) => {$(
        impl From<$ty> for ToChannelSys {
            fn from(v: $ty) -> Self { Self::$variant(v) }
        }
    )*};
}

into_to_sys! {
    OpenRequest => Open,
    CloseRequest => Close,
    ExpireRequest => Expire,
    HubNotice => Hub,
    ReadyRequest => Ready,
    SyncRequest => Sync,
    StatusRequest => Status,
    ThresholdsRequest => Thresholds,
    HostRequest => Host,
    EmitRequest => Emit,
    SendRequest => Send,
    StoppedNotice => Stopped,
}

/// A request's answer type, taken back out of `FromChannelSys`.
macro_rules! answer_of {
    ($($ty:ty => $variant:ident),* $(,)?) => {$(
        impl TryFrom<FromChannelSys> for $ty {
            type Error = subsys::SubsysError;

            fn try_from(v: FromChannelSys) -> subsys::SubsysResult<Self> {
                match v {
                    FromChannelSys::$variant(answer) => Ok(answer),
                    _ => Err(subsys::SubsysError::ResponseType),
                }
            }
        }
    )*};
}

answer_of! {
    Opened => Opened,
    Closed => Closed,
    Result<ChannelEpoch, ChannelVerifyError> => Ready,
    Result<Synced, ChannelSyncError> => Synced,
    ChannelStatus => Status,
    Result<SpamThresholdsData, String> => Thresholds,
    Result<(), ChannelSendError> => Hosted,
    Result<String, EmitError> => Emitted,
    SendVerdict => Sent,
}

/// Each request's answer type.
macro_rules! request_of {
    ($($ty:ty => $answer:ty),* $(,)?) => {$(
        impl subsys::Request<ChannelSystem> for $ty {
            type ResponseType = $answer;
        }
    )*};
}

request_of! {
    OpenRequest => Opened,
    CloseRequest => Closed,
    ExpireRequest => Closed,
    ReadyRequest => Result<ChannelEpoch, ChannelVerifyError>,
    SyncRequest => Result<Synced, ChannelSyncError>,
    StatusRequest => ChannelStatus,
    ThresholdsRequest => Result<SpamThresholdsData, String>,
    HostRequest => Result<(), ChannelSendError>,
    EmitRequest => Result<String, EmitError>,
    SendRequest => SendVerdict,
}
