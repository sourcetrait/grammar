use crate::*;

/// The model-path prefix RESERVED for the local host's own packets.
pub const MCP_RESERVED_PREFIX: &str = "mcp/";

/// The model-path prefix RESERVED for the host's packets about a remote.
pub const REMOTE_RESERVED_PREFIX: &str = "remote/";

/// The reserved prefix `model` lies under, if any.
pub fn reserved_prefix(model: &str) -> Option<&'static str> {
    [MCP_RESERVED_PREFIX, REMOTE_RESERVED_PREFIX]
        .into_iter()
        .find(|prefix| model.starts_with(prefix))
}

/// The usable frame maximum.
pub const MAX_FRAME_BYTES: usize = 1_048_575;

/// The most packets a sync Channel queues; past it, the oldest is dropped.
pub const SYNC_QUEUE_CAP: usize = 1000;

/// The client's task-notification cap on a rendered packet line, in characters.
pub const NOTIFICATION_CAP: usize = 500;

/// The event key marking a spilled event, its value the full event's path.
pub const EVENT_SPILL_KEY: &str = "spilled_event_path";

/// The event key a spilled event's size in bytes rides under.
pub const EVENT_BYTES_KEY: &str = "event_bytes";

/// The dest a remote event spill transfers under, in its delivery's directory.
pub const EVENT_SPILL_DEST: &str = ".mcp_event.nuon";

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
    /// A Monitor reads them from the hub's loopback WSS stream as they come.
    Async,
    /// They queue until the agent takes them by `interface channel sync`.
    Sync,
}

impl ChannelMode {
    /// The mode's name, as `interface channel open` takes it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Async => "async",
            Self::Sync => "sync",
        }
    }

    /// The mode a name names, if any.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "async" => Some(Self::Async),
            "sync" => Some(Self::Sync),
            _ => None,
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
    pub event: NuVal,
    pub attached: Option<NuVal>,
}

/// A body's send to a linked peer as data.
#[cereal::derived(Data)]
pub struct RemoteSendData {
    pub mcp_nom: String,
    pub model: String,
    pub event: NuVal,
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
