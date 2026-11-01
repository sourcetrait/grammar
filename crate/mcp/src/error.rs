use crate::*;

pub type GrammarMcpResult<T> = Result<T, GrammarMcpError>;

#[derive(Debug, Clone)]
pub enum GrammarMcpError {
    /// What the engine refused or failed at, its kinds and messages its own.
    Engine(EngineError),
    /// What the transport to a worker failed at, under its `daemon::` kinds.
    Transport(TransportError),
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

impl GrammarMcpError {
    /// The portable runtime error this is; none for an engine or transport error.
    pub(crate) fn as_runtime(&self) -> Option<RuntimeError> {
        Some(match self.clone() {
            Self::Engine(_) | Self::Transport(_) => return None,
            Self::InterfaceViolations { diagnostics } => {
                RuntimeError::InterfaceViolations { diagnostics }
            }
            Self::HostUnknown { host } => RuntimeError::HostUnknown { host },
            Self::SchemaInvalid { reason } => RuntimeError::SchemaInvalid { reason },
            Self::PurviewInvalidId { id, reason } => RuntimeError::PurviewInvalidId { id, reason },
            Self::ChannelStart { reason } => RuntimeError::ChannelStart { reason },
            Self::ChannelNotOpen => RuntimeError::ChannelNotOpen,
            Self::ChannelNotClaimed { mode } => RuntimeError::ChannelNotClaimed { mode },
            Self::ChannelWrongMode { open, asked } => RuntimeError::ChannelWrongMode { open, asked },
            Self::ChannelPeerGone => RuntimeError::ChannelPeerGone,
            Self::RemoteInvalidParams { reason } => RuntimeError::RemoteInvalidParams { reason },
            Self::RemoteAlreadyOpen { alias } => RuntimeError::RemoteAlreadyOpen { alias },
            Self::RemoteNotOpen { alias } => RuntimeError::RemoteNotOpen { alias },
            Self::RemoteOpenFailed { alias, reason } => {
                RuntimeError::RemoteOpenFailed { alias, reason }
            }
            Self::RemoteChannelUnverified => RuntimeError::RemoteChannelUnverified,
            Self::RemotesFile { reason } => RuntimeError::RemotesFile { reason },
            Self::Internal { phase, reason } => RuntimeError::Internal { phase, reason },
        })
    }
}

impl From<RuntimeError> for GrammarMcpError {
    fn from(e: RuntimeError) -> Self {
        match e {
            RuntimeError::InterfaceViolations { diagnostics } => {
                Self::InterfaceViolations { diagnostics }
            }
            RuntimeError::HostUnknown { host } => Self::HostUnknown { host },
            RuntimeError::SchemaInvalid { reason } => Self::SchemaInvalid { reason },
            RuntimeError::PurviewInvalidId { id, reason } => Self::PurviewInvalidId { id, reason },
            RuntimeError::ChannelStart { reason } => Self::ChannelStart { reason },
            RuntimeError::ChannelNotOpen => Self::ChannelNotOpen,
            RuntimeError::ChannelNotClaimed { mode } => Self::ChannelNotClaimed { mode },
            RuntimeError::ChannelWrongMode { open, asked } => Self::ChannelWrongMode { open, asked },
            RuntimeError::ChannelPeerGone => Self::ChannelPeerGone,
            RuntimeError::RemoteInvalidParams { reason } => Self::RemoteInvalidParams { reason },
            RuntimeError::RemoteAlreadyOpen { alias } => Self::RemoteAlreadyOpen { alias },
            RuntimeError::RemoteNotOpen { alias } => Self::RemoteNotOpen { alias },
            RuntimeError::RemoteOpenFailed { alias, reason } => {
                Self::RemoteOpenFailed { alias, reason }
            }
            RuntimeError::RemoteChannelUnverified => Self::RemoteChannelUnverified,
            RuntimeError::RemotesFile { reason } => Self::RemotesFile { reason },
            RuntimeError::Internal { phase, reason } => Self::Internal { phase, reason },
        }
    }
}

impl From<TransportError> for GrammarMcpError {
    fn from(e: TransportError) -> Self {
        GrammarMcpError::Transport(e)
    }
}

impl From<io::Error> for GrammarMcpError {
    fn from(e: io::Error) -> Self {
        GrammarMcpError::Internal {
            phase: "io".to_string(),
            reason: e.to_string(),
        }
    }
}

impl From<EngineError> for GrammarMcpError {
    fn from(e: EngineError) -> Self {
        GrammarMcpError::Engine(e)
    }
}

/// A failure, `{nonce, error}`: the error's rows, its warnings dropped.
pub(crate) fn error_to_call_result(
    error: impl Into<GrammarMcpError>,
    nonce: datum::Nonce,
) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    let rows = match error.into() {
        GrammarMcpError::Engine(error) => error.rows(),
        GrammarMcpError::Transport(error) => error.rows(),
        runtime => runtime.as_runtime().map(RuntimeError::rows).unwrap_or_default(),
    };
    envelope_text(nonce, "error", serde_to_nu(&rows)?)
}
