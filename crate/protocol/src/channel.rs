use crate::*;

/// The model-path prefix RESERVED for the local host's own packets.
pub const MCP_RESERVED_PREFIX: &str = "mcp/";

/// The model-path prefix RESERVED for the host's packets about a remote.
pub const REMOTE_RESERVED_PREFIX: &str = "remote/";

/// The model-path prefix RESERVED for the host's messages between agents.
pub const CHANNEL_RESERVED_PREFIX: &str = "channel/";

/// The standard message between agents, which the host alone stamps.
pub const CHAT_MODEL: &str = "channel/Chat";

/// The file every chat attaches, its message's text.
pub const CHAT_MESSAGE_FILE: &str = "message.md";

/// The most characters a chat's summary takes.
pub const CHAT_SUMMARY_MAX_CHARS: usize = 80;

/// The reserved prefix `model` lies under, if any.
pub fn reserved_prefix(model: &str) -> Option<&'static str> {
    [MCP_RESERVED_PREFIX, REMOTE_RESERVED_PREFIX, CHANNEL_RESERVED_PREFIX]
        .into_iter()
        .find(|prefix| model.starts_with(prefix))
}

/// The reserved prefix a peer's relay is refused under: `channel/` passes.
///
/// A peer's host stamped its `channel/` model, as the link trusts it by its key.
pub fn relay_reserved_prefix(model: &str) -> Option<&'static str> {
    reserved_prefix(model).filter(|prefix| *prefix != CHANNEL_RESERVED_PREFIX)
}

/// The model-path prefix of an await job's events, under the host's own.
pub const AWAIT_MODEL_PREFIX: &str = "mcp/await/";

/// An await job's beat: status's value or its error's rows.
pub const AWAIT_STATUS_MODEL: &str = "mcp/await/Status";

/// An await job's end: execute's value or its error's rows.
pub const AWAIT_RESULT_MODEL: &str = "mcp/await/Result";

/// An await job's interruption: stop's value or its error's rows, and why it ran.
pub const AWAIT_STOP_MODEL: &str = "mcp/await/Stop";

/// An await job's event for the Channel, a host packet's model and data.
#[cereal::derived(Data)]
pub struct AwaitEvent {
    pub model: String,
    pub data: NuVal,
}

/// The most characters a packet's model takes, so a spilled line fits the cap.
pub const MODEL_MAX_CHARS: usize = 128;

/// Whether `model` is past `MODEL_MAX_CHARS`, which a sender and a relay refuse.
pub fn model_too_long(model: &str) -> bool {
    model.chars().count() > MODEL_MAX_CHARS
}

/// The refusal of a model past `MODEL_MAX_CHARS`.
pub fn model_too_long_reason() -> String {
    format!("a model is at most {MODEL_MAX_CHARS} characters")
}

/// The usable frame maximum.
pub const MAX_FRAME_BYTES: usize = 1_048_575;

/// The most packets a sync Channel queues; past it, the oldest is dropped.
pub const SYNC_QUEUE_CAP: usize = 1000;

/// The client's task-notification cap on a rendered packet line, in characters.
pub const NOTIFICATION_CAP: usize = 500;

/// What an emit is allowed to do right now.
#[cereal::derived(Data, Copy, Eq)]
pub enum ChannelPhase {
    /// No hub running; the ordinary state of a session that never opens one.
    Closed,
    /// The hub is up but the peer has not yet proven it owns the session.
    Open,
    /// `interface channel ready` succeeded; emits flow.
    Verified,
}

/// How an open Channel's packets reach its peer.
#[cereal::derived(Data, Copy, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChannelMode {
    /// They reach the peer as they come, by the face the open chose.
    Async,
    /// They queue until the agent takes them by `interface channel sync`.
    Sync,
}

impl ChannelMode {
    /// The mode's name, as `interface channel open` answers it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Async => "async",
            Self::Sync => "sync",
        }
    }
}

/// One logical Channel's generation, which an open begins and a close ends.
#[cereal::derived(Data, Copy, Eq)]
pub struct ChannelEpoch(pub u64);

/// The logical Channel as its system holds it.
#[cereal::derived(Data)]
pub struct ChannelState {
    pub phase: ChannelPhase,
    pub mode: Option<ChannelMode>,
    pub epoch: ChannelEpoch,
    pub claimed: bool,
    /// Packets the sync queue dropped past its cap since the last sync.
    pub sync_dropped: u64,
}

/// A body's Channel send as data, the eval's stop held back on its own side.
#[cereal::derived(Data)]
pub struct ChannelSendData {
    /// The sending call's origin, `thread/<nonce>`.
    pub from: String,
    pub model: String,
    pub data: NuVal,
    pub attached: Option<NuVal>,
}

/// A body's send to a linked peer as data.
#[cereal::derived(Data)]
pub struct RemoteSendData {
    pub mcp_nom: String,
    pub model: String,
    pub data: NuVal,
    /// The files table, `src` beside `dest`; `None` for a send without files.
    pub attached: Option<NuVal>,
}

/// The Channel's send-rate thresholds, as `interface channel config` shows them.
#[cereal::derived(Data, Copy, Eq)]
pub struct SpamThresholdsData {
    pub warn_window_secs: u64,
    pub warn_rate: u32,
    pub error_window_secs: u64,
    pub error_rate: u32,
}
