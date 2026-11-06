pub(crate) mod claude;
pub(crate) mod codex;
#[cfg(test)]
mod tests {
    mod codex;
    mod harness;
}

pub(crate) use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub(crate) mod ser {
    pub(crate) use ::serde::Deserialize;
}

pub use crate::{
    claude::CLAUDE_CLIENT,
    codex::{CODEX_CLIENT, DaemonStatus, THREAD_ID_META, daemon_status, parse_daemon_status},
};

#[cfg(unix)]
pub use crate::codex::client::{AppServerClient, ToolOutput};

/// The harness a client is, by the name its `initialize` gives in `clientInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Harness {
    /// Claude Code, its Channel the hub's wss stream a Monitor reads.
    Claude,
    /// Codex, its Channel delivered into a thread through its app-server.
    Codex,
    /// Any other client, its Channel the sync one `interface channel sync` reads.
    Other(String),
}

impl Harness {
    /// The harness a `clientInfo` name names.
    pub fn of(client: &str) -> Self {
        match client {
            CLAUDE_CLIENT => Self::Claude,
            CODEX_CLIENT => Self::Codex,
            other => Self::Other(other.to_string()),
        }
    }
}
