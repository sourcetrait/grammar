//! A session after its hellos: each side's frames, the requests, and the asks.
use crate::*;

/// How often the MCP pings a session.
pub const PING_INTERVAL: std::time::Duration = std::time::Duration::from_secs(10);

/// How long either side waits for a frame before it ends the session.
pub const SESSION_IDLE: std::time::Duration = std::time::Duration::from_secs(30);

/// How long grammar-ssh's ask waits for the MCP's answer.
pub const ASK_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);

/// How long a session's end waits for its calls to stop.
pub const DRAIN_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

/// A liveness probe, its nonce echoed in the answer.
#[cereal::derived(Data, Copy, Eq)]
pub struct Ping {
    pub nonce: u64,
}

/// A session's orderly end, with why where its sender wasn't asked to end it.
#[cereal::derived(Data)]
pub struct SessionClose {
    pub error: Option<TransportError>,
}

/// What the MCP sends on a session after its hello.
#[cereal::derived(Data)]
pub enum McpFrame {
    /// A request of grammar-ssh's engine, answered once under its id.
    Request { id: u64, request: EngineRequest },
    /// The answer to grammar-ssh's ask under its id.
    Answer { id: u64, answer: HostAnswer },
    Ping(Ping),
    Close(SessionClose),
}

/// What grammar-ssh sends on a session after its reply.
#[cereal::derived(Data)]
pub enum SshFrame {
    /// The answer to the MCP's request under its id.
    Response { id: u64, response: EngineResponse },
    /// An eval's ask of the MCP, answered once under its id.
    Ask { id: u64, ask: SessionAsk },
    /// An emergency grammar-ssh's operator raised.
    Emergency(Emergency),
    Pong(Ping),
    Close(SessionClose),
}

/// A tool call grammar-ssh serves, its parameters as the MCP's tool took them.
#[cereal::derived(Data)]
pub enum EngineRequest {
    Execute(ExecuteRequest),
    Rerun(RerunRequest),
    Call(CallRequest),
    Bed(BedRequest),
    Interface(InterfaceRequest),
}

impl EngineRequest {
    /// Whether the call runs an eval, which takes a call's slot.
    pub fn is_call(&self) -> bool {
        !matches!(self, Self::Interface(_))
    }
}

impl From<ExecuteRequest> for EngineRequest {
    fn from(v: ExecuteRequest) -> Self { Self::Execute(v) }
}

impl From<RerunRequest> for EngineRequest {
    fn from(v: RerunRequest) -> Self { Self::Rerun(v) }
}

impl From<CallRequest> for EngineRequest {
    fn from(v: CallRequest) -> Self { Self::Call(v) }
}

impl From<BedRequest> for EngineRequest {
    fn from(v: BedRequest) -> Self { Self::Bed(v) }
}

impl From<InterfaceRequest> for EngineRequest {
    fn from(v: InterfaceRequest) -> Self { Self::Interface(v) }
}

/// grammar-ssh's answer to a tool call.
#[cereal::derived(Data)]
pub enum EngineResponse {
    /// The call's value, or the rows its error reports.
    Answer(CallAnswer),
    /// The request refused before the engine took it, or its answer past the frame.
    Refused(TransportError),
}

/// A tool call's answer: its nonce, and its value or the rows its error reports.
#[cereal::derived(Data)]
pub struct CallAnswer {
    pub nonce: datum::Nonce,
    pub result: Result<NuVal, Vec<Diagnostic>>,
}

/// A host's identity, homes, and plugins, as `interface info` reports them.
#[cereal::derived(Data)]
pub struct HostInfo {
    pub version: String,
    pub nu_version: String,
    pub id: String,
    pub namespace: String,
    /// The host's nom, its `$env.EQUIP_MCP_NOM`.
    pub nom: String,
    /// The host's working directory, its `$env.EQUIP_WORK_DIR`.
    pub work_dir: String,
    /// The host's bulk scratch directory, its `$env.EQUIP_TMP`.
    pub tmp: String,
    /// The host's transient directories, an agent's and its own.
    pub transient: TransientPaths,
    pub plugins: Vec<PluginInfo>,
}

/// The runtime directory's child holding each host id's agent scratch.
pub const AI_TRANSIENT_DIR: &str = "ai";

/// A host's two transient directories, as `interface info` reports them.
#[cereal::derived(Data, Eq)]
pub struct TransientPaths {
    /// An agent's transient scratch for the host's id, `<runtime>/ai/<id>`.
    pub ai: String,
    /// The host's own transient directory, its `$env.EQUIP_TRANSIENT`.
    pub grammar: String,
}

/// One plugin as a positional `[name, version]` pair, its version null if unknown.
#[cereal::derived(Data)]
pub struct PluginInfo(pub String, pub Option<String>);

/// An eval's ask of the MCP whose session grammar-ssh serves.
#[cereal::derived(Data)]
pub enum SessionAsk {
    /// A body's packet for the MCP's Channel.
    ChannelSend(ChannelSendData),
    /// The stop a spam verdict made, for the error's emergency.
    Stopped(ChannelSpamErrorEmergency),
    /// A body's message for a peer the MCP links, its files read by grammar-ssh.
    RemoteSend(PreparedSend),
    /// The MCP's configuration in force.
    Config,
}

/// A remote send as its sender prepares it: its data and each file's bytes.
#[cereal::derived(Data)]
pub struct PreparedSend {
    pub mcp_nom: String,
    pub model: String,
    pub data: NuVal,
    pub files: Vec<SendFile>,
}

/// One file of a remote send: where the peer puts it, and its bytes.
#[cereal::derived(Data)]
pub struct SendFile {
    pub dest: String,
    pub bytes: Vec<u8>,
}

/// The host's answer to an ask.
#[cereal::derived(Data)]
pub enum HostAnswer {
    /// Sent, under the message's id.
    Sent(String),
    /// Refused, and why.
    Refused(String),
    /// Over the hard threshold: refused, the sender's eval to be stopped.
    Stopped(HostStop),
    /// The configuration in force, mirroring the TOML.
    Config(NuVal),
    /// The pin is held.
    Pinned,
    /// The stop is reported.
    Noted,
}

/// A spam verdict's stop, as its sender acts on it and reports it.
#[cereal::derived(Data)]
pub struct HostStop {
    pub hits: u32,
    pub window_secs: u64,
    pub rate: u32,
    /// Whether the stop begins an episode, the one its emergency reports.
    pub notify: bool,
    /// The refusal the sender answers with.
    pub reason: String,
}
