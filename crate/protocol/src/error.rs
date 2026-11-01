use crate::*;

/// The host name reserved for this MCP's own engine.
pub const LOCAL_HOST: &str = "local";

/// What the runtime refused or failed at, outside the engine's own refusals.
#[cereal::derived(Data)]
pub enum RuntimeError {
    InterfaceViolations {
        diagnostics: Vec<Diagnostic>,
    },
    HostUnknown {
        host: String,
    },
    SchemaInvalid {
        reason: String,
    },
    PurviewInvalidId {
        id: String,
        reason: String,
    },
    ChannelStart {
        reason: String,
    },
    ChannelNotOpen,
    ChannelNotClaimed {
        mode: ChannelMode,
    },
    ChannelWrongMode {
        open: ChannelMode,
        asked: ChannelMode,
    },
    ChannelPeerGone,
    RemoteInvalidParams {
        reason: String,
    },
    RemoteAlreadyOpen {
        alias: String,
    },
    RemoteNotOpen {
        alias: String,
    },
    RemoteOpenFailed {
        alias: String,
        reason: String,
    },
    RemoteChannelUnverified,
    RemotesFile {
        reason: String,
    },
    Internal {
        phase: String,
        reason: String,
    },
}

impl RuntimeError {
    /// The kind a single-condition error reports under.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::HostUnknown { .. } => "host::unknown",
            Self::SchemaInvalid { .. } => "schema::invalid",
            Self::PurviewInvalidId { .. } => "purview::invalid_id",
            Self::ChannelStart { .. } => "channel::start",
            Self::ChannelNotOpen => "channel::not_open",
            Self::ChannelNotClaimed { .. } => "channel::not_claimed",
            Self::ChannelWrongMode { .. } => "channel::wrong_mode",
            Self::ChannelPeerGone => "channel::peer_gone",
            Self::RemoteInvalidParams { .. } => "remote::channel::invalid_params",
            Self::RemoteAlreadyOpen { .. } => "remote::channel::already_open",
            Self::RemoteNotOpen { .. } => "remote::channel::not_open",
            Self::RemoteOpenFailed { .. } => "remote::channel::open_failed",
            Self::RemoteChannelUnverified => "remote::channel::unverified",
            Self::RemotesFile { .. } => "remote::file",
            Self::Internal { .. } => "internal",
            Self::InterfaceViolations { .. } => {
                unreachable!("a violations error renders through its rows, not a kind")
            }
        }
    }

    /// The message a single-condition error carries.
    pub fn message(&self) -> String {
        match self {
            Self::HostUnknown { host } => format!(
                "unknown host `{host}`: `{LOCAL_HOST}` is this MCP's own host, \
                 and no other is configured"
            ),
            Self::SchemaInvalid { reason } => format!("invalid schema: {reason}"),
            Self::PurviewInvalidId { id, reason } => {
                format!("invalid purview id `{id}`: {reason}")
            }
            Self::ChannelStart { reason } => format!("cannot start the channel hub: {reason}"),
            Self::ChannelNotOpen => {
                "the channel is not open; run `interface channel open` first".to_string()
            }
            Self::ChannelNotClaimed {
                mode: ChannelMode::Async,
            } => "nothing has connected to the channel yet; start \
                 a Monitor at the wss endpoint `interface channel open` returned, see the \
                 channel/Open packet, then run `interface channel ready`"
                .to_string(),
            Self::ChannelNotClaimed {
                mode: ChannelMode::Sync,
            } => "nothing has taken the channel's packets yet; run `interface channel \
                 sync`, see the channel/Open packet, then run `interface channel ready`"
                .to_string(),
            Self::ChannelWrongMode { open, asked } => format!(
                "the channel is open in `{}` mode, not `{}`; close it with \
                 `interface channel close` to open it in `{}`",
                open.name(),
                asked.name(),
                asked.name(),
            ),
            Self::ChannelPeerGone => "the channel's peer connection is gone; run \
                 `interface channel close` then `interface channel open` for a fresh channel"
                .to_string(),
            Self::RemoteInvalidParams { reason } => {
                format!("invalid remote link parameters: {reason}")
            }
            Self::RemoteAlreadyOpen { alias } => format!("remote link `{alias}` is already open"),
            Self::RemoteNotOpen { alias } => format!("no open remote link `{alias}`"),
            Self::RemoteOpenFailed { alias, reason } => {
                format!("remote link `{alias}` failed to open: {reason}")
            }
            Self::RemoteChannelUnverified => "the channel is not verified; a remote link's \
                 notices ride it, so open and verify the channel before the link"
                .to_string(),
            Self::RemotesFile { reason } => format!("the remotes file: {reason}"),
            Self::Internal { phase, reason } => format!("internal error [{phase}]: {reason}"),
            Self::InterfaceViolations { .. } => {
                unreachable!("a violations error renders through its rows, not a message")
            }
        }
    }

    /// The error rows a failure reports: a violations error's errors, else one.
    pub fn rows(self) -> Vec<Diagnostic> {
        match self {
            Self::InterfaceViolations { diagnostics } => Diagnostic::bucket(diagnostics).0,
            single => vec![Diagnostic::error(single.kind(), None, single.message())],
        }
    }
}

/// Why a session to a worker failed, apart from what the worker's engine answered.
#[cereal::derived(Data)]
pub enum TransportError {
    VersionMismatch { expected: String, found: String },
    SchemaMismatch { expected: String, found: String },
    CodecMismatch { expected: String, found: String },
    WrongWorker { expected: String, found: String },
    /// The host refused the connection, or never answered it.
    ConnectFailed { reason: String },
    /// The host presented a key none of its pinned keys match.
    HostKeyRejected { reason: String },
    /// The SSH session never authenticated, so nothing was sent.
    AuthenticationFailed { reason: String },
    /// The worker couldn't start or serve, its own reason given.
    Unavailable { reason: String },
    /// The session's requests of a kind are at their bound; the request didn't run.
    Busy,
    InvalidFrame { reason: String },
    ResultTooLarge { limit: u64 },
    /// The session ended with the request in flight: its outcome is unknown.
    SessionLost { reason: String },
}

impl TransportError {
    /// The stable kind the frontend's envelope reports.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::VersionMismatch { .. } => "daemon::version_mismatch",
            Self::SchemaMismatch { .. } => "daemon::schema_mismatch",
            Self::CodecMismatch { .. } => "daemon::codec_mismatch",
            Self::WrongWorker { .. } => "daemon::wrong_worker",
            Self::ConnectFailed { .. } => "daemon::connect_failed",
            Self::HostKeyRejected { .. } => "daemon::host_key_rejected",
            Self::AuthenticationFailed { .. } => "daemon::authentication_failed",
            Self::Unavailable { .. } => "daemon::unavailable",
            Self::Busy => "daemon::busy",
            Self::InvalidFrame { .. } => "daemon::invalid_frame",
            Self::ResultTooLarge { .. } => "daemon::result_too_large",
            Self::SessionLost { .. } => "daemon::session_lost",
        }
    }

    /// The message the frontend's envelope reports.
    pub fn message(&self) -> String {
        match self {
            Self::VersionMismatch { expected, found } => {
                format!("protocol version {found} where {expected} is required")
            }
            Self::SchemaMismatch { expected, found } => {
                format!("protocol schema {found} where {expected} is required")
            }
            Self::CodecMismatch { expected, found } => {
                format!("codec {found} where {expected} is required")
            }
            Self::WrongWorker { expected, found } => {
                format!("the worker is {found}, not the expected {expected}")
            }
            Self::ConnectFailed { reason } => format!("the host couldn't be reached: {reason}"),
            Self::HostKeyRejected { reason } => {
                format!("the host's key matches none pinned for it; nothing was sent: {reason}")
            }
            Self::AuthenticationFailed { reason } => {
                format!("authentication failed before any request was sent: {reason}")
            }
            Self::Unavailable { reason } => format!("the worker is unavailable: {reason}"),
            Self::Busy => "the session's requests are at their bound; nothing ran".to_string(),
            Self::InvalidFrame { reason } => format!("an invalid frame: {reason}"),
            Self::ResultTooLarge { limit } => {
                format!("the result exceeds the {limit}-byte limit")
            }
            Self::SessionLost { reason } => {
                format!("the session ended with the request in flight, its outcome unknown: {reason}")
            }
        }
    }

    /// The one error row the failure reports.
    pub fn rows(self) -> Vec<Diagnostic> {
        vec![Diagnostic::error(self.kind(), None, self.message())]
    }
}
