//! The remote Channel's portable data: commands' results, events, and relays.
use crate::*;

/// The model a link's packet rides under once both its connections pair.
pub const MODEL_CONNECTED: &str = "remote/channel/Connected";

/// The model a link's packet rides under when it ends or never pairs.
pub const MODEL_DISCONNECTED: &str = "remote/channel/Disconnected";

/// The model of a send's report once its message is written to the link.
pub const MODEL_SENT: &str = "remote/channel/Sent";

/// The model of a send's report when it failed before or during its write.
pub const MODEL_UNSENT: &str = "remote/channel/Unsent";

/// The longest nom or delivery id a peer may name, in bytes.
pub const PEER_ID_MAX: usize = 64;

/// The seven remote Channel commands, in the catalog's order.
pub const REMOTE_COMMANDS: [CommandKind; 7] = [
    CommandKind::RemoteList,
    CommandKind::RemoteGet,
    CommandKind::RemoteSet,
    CommandKind::RemoteUnset,
    CommandKind::RemoteListen,
    CommandKind::RemoteConnect,
    CommandKind::RemoteClose,
];

/// Whether `text` is a nom or a delivery id a peer may name: base62 alone.
pub fn valid_peer_id(text: &str) -> bool {
    !text.is_empty() && text.len() <= PEER_ID_MAX && text.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// Whether a send's dest is a relative path of plain components, never an escape.
pub fn safe_dest(dest: &str) -> bool {
    let path = std::path::Path::new(dest);
    !dest.is_empty() && path.is_relative() && path.components().all(|c| matches!(c, std::path::Component::Normal(_)))
}

/// A remote alias's state: closed, listening for its peer, or open.
#[cereal::derived(Data, Copy, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RemoteLinkState {
    Closed,
    Listening,
    Open,
}

/// One remote in `interface remote channel list` and `get`: its entry and state.
#[cereal::derived(Data, Eq)]
pub struct RemoteChannelRow {
    pub alias: String,
    /// The `ip:port` a connect dials by default, as the file writes it.
    pub address: Option<String>,
    /// The `ip` or `ip:port` a listen binds by default, as the file writes it.
    pub listen: Option<String>,
    /// The peer's certificate, as the file writes it.
    pub cert: Option<String>,
    pub state: RemoteLinkState,
    /// The address bound or dialed while the remote is live.
    pub endpoint: Option<String>,
    /// The peer's nom while a link is open.
    pub remote_mcp_nom: Option<String>,
}

/// `interface remote channel listen`'s answer: the address it bound.
#[cereal::derived(Data, Eq)]
pub struct RemoteListenResult {
    /// The address bound, its port the kernel's when random.
    pub endpoint: String,
}

/// A remote Channel command's result, as its tool renders it.
#[cereal::derived(Data, Eq)]
pub enum RemoteResult {
    /// `list`: every remote, configured and live.
    Rows(Vec<RemoteChannelRow>),
    /// `get` and `set`: one remote's entry and state.
    Row(RemoteChannelRow),
    /// `listen`: the address it bound.
    Listening(RemoteListenResult),
    /// `unset`, `connect`, and `close`, which answer nothing.
    Done,
}

impl RemoteResult {
    /// Whether this is the result `kind`, a remote Channel command, answers.
    pub fn answers(&self, kind: CommandKind) -> bool {
        matches!(
            (kind, self),
            (CommandKind::RemoteList, Self::Rows(_))
                | (CommandKind::RemoteGet | CommandKind::RemoteSet, Self::Row(_))
                | (CommandKind::RemoteListen, Self::Listening(_))
                | (CommandKind::RemoteUnset | CommandKind::RemoteConnect | CommandKind::RemoteClose, Self::Done)
        )
    }
}

/// Why a link or a send failed: its kind and its message.
#[cereal::derived(Data, Eq)]
pub struct RemoteFailure {
    pub kind: String,
    pub message: String,
}

/// A link's lifecycle and a send's delivery, as their packets report them.
#[cereal::derived(Data, Eq)]
pub enum RemoteChannelEvent {
    Connected {
        remote: String,
        mcp_nom: String,
    },
    Disconnected {
        remote: String,
        mcp_nom: Option<String>,
        error: Option<RemoteFailure>,
    },
    Sent {
        id: String,
        mcp_nom: String,
    },
    Unsent {
        id: String,
        mcp_nom: String,
        error: RemoteFailure,
    },
}

impl RemoteChannelEvent {
    /// The model its packet rides under.
    pub fn model(&self) -> &'static str {
        match self {
            Self::Connected { .. } => MODEL_CONNECTED,
            Self::Disconnected { .. } => MODEL_DISCONNECTED,
            Self::Sent { .. } => MODEL_SENT,
            Self::Unsent { .. } => MODEL_UNSENT,
        }
    }

    /// The record its packet carries, a key present only where it has a value.
    pub fn event(&self) -> NuVal {
        let text = |s: &str| NuVal::String(s.to_string());
        let failure = |f: &RemoteFailure| {
            NuVal::Record(vec![("kind".to_string(), text(&f.kind)), ("message".to_string(), text(&f.message))])
        };
        let fields = match self {
            Self::Connected { remote, mcp_nom } => vec![("remote", Some(text(remote))), ("mcp_nom", Some(text(mcp_nom)))],
            Self::Disconnected { remote, mcp_nom, error } => vec![
                ("remote", Some(text(remote))),
                ("mcp_nom", mcp_nom.as_deref().map(text)),
                ("error", error.as_ref().map(failure)),
            ],
            Self::Sent { id, mcp_nom } => vec![("id", Some(text(id))), ("mcp_nom", Some(text(mcp_nom)))],
            Self::Unsent { id, mcp_nom, error } => vec![
                ("id", Some(text(id))),
                ("mcp_nom", Some(text(mcp_nom))),
                ("error", Some(failure(error))),
            ],
        };
        NuVal::Record(fields.into_iter().filter_map(|(key, value)| Some((key.to_string(), value?))).collect())
    }
}

/// A peer's message as a link relays it.
#[cereal::derived(Data)]
pub struct RemotePacket {
    /// The delivery's id, as its sender minted it.
    pub delivery: String,
    /// The sending peer's nom.
    pub origin: String,
    pub model: String,
    /// The payload, data alone.
    pub event: NuVal,
}

impl RemotePacket {
    /// The packet held to what a relay keeps: base62 ids, an unreserved model.
    pub fn check(&self) -> Result<(), TransportError> {
        let refused = |reason: String| Err(TransportError::InvalidFrame { reason });
        if !valid_peer_id(&self.origin) {
            return refused("a relayed packet whose origin isn't a base62 nom".to_string());
        }
        if !valid_peer_id(&self.delivery) {
            return refused("a relayed packet whose delivery id isn't base62".to_string());
        }
        if self.model.is_empty() {
            return refused("a relayed packet with no model".to_string());
        }
        if let Some(prefix) = reserved_prefix(&self.model) {
            return refused(format!("a relayed model under the host's `{prefix}` reservation"));
        }
        Ok(())
    }

    /// The `from` its packet names: `remote/channel/<nom>`.
    pub fn packet_from(&self) -> String {
        format!("{REMOTE_RESERVED_PREFIX}channel/{}", self.origin)
    }
}
