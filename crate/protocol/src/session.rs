//! A session after its hellos: each side's frames, the requests, and the asks.
use crate::*;

/// How often the MCP pings a session.
pub const PING_INTERVAL: std::time::Duration = std::time::Duration::from_secs(10);

/// How long either side waits for a frame before it ends the session.
pub const SESSION_IDLE: std::time::Duration = std::time::Duration::from_secs(30);

/// How long a worker's ask waits for the MCP's answer.
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
    /// A request of the worker's engine, answered once under its id.
    Request { id: u64, request: EngineRequest },
    /// The answer to the worker's ask under its id.
    Answer { id: u64, answer: HostAnswer },
    Ping(Ping),
    Close(SessionClose),
}

/// What the worker sends on a session after its reply.
#[cereal::derived(Data)]
pub enum WorkerFrame {
    /// The answer to the MCP's request under its id.
    Response { id: u64, response: EngineResponse },
    /// An eval's ask of the MCP, answered once under its id.
    Ask { id: u64, ask: SessionAsk },
    /// An emergency the worker's operator raised.
    Emergency(Emergency),
    Pong(Ping),
    Close(SessionClose),
}

/// A request of the worker's engine.
#[cereal::derived(Data)]
pub enum EngineRequest {
    /// An eval, a rerun, a call, or a control command.
    Wire(ToEngineSys),
    /// One of the namespace's purview requests.
    Purview(PurviewRequest),
    /// The worker's identity, homes, and plugins.
    Info,
}

impl EngineRequest {
    /// Whether the request is an eval, a rerun, or a call, which take a call's slot.
    pub fn is_call(&self) -> bool {
        matches!(self, Self::Wire(ToEngineSys::Execute(_) | ToEngineSys::Rerun(_) | ToEngineSys::Call(_)))
    }
}

/// The worker's answer to a request.
#[cereal::derived(Data)]
pub enum EngineResponse {
    Wire(FromEngineSys),
    Purview(PurviewAnswer),
    Info(WorkerInfo),
    /// The request refused before the engine took it, or its answer past the frame.
    Refused(TransportError),
}

/// A worker's identity, homes, and plugins, as `interface info` reports its host's.
#[cereal::derived(Data)]
pub struct WorkerInfo {
    pub version: String,
    pub nu_version: String,
    pub id: String,
    pub namespace: String,
    /// The worker's nom, its `$env.EQUIP_MCP_NOM`.
    pub nom: String,
    /// The worker's working directory, its `$env.EQUIP_WORK_DIR`.
    pub work_dir: String,
    /// The worker's bulk scratch directory, its `$env.EQUIP_TMP`.
    pub tmp: String,
    /// The worker's transient scratch directory, its `$env.EQUIP_TRANSIENT`.
    pub transient: String,
    pub plugins: Vec<PluginInfo>,
}

/// One plugin as a positional `[name, version]` pair, its version null if unknown.
#[cereal::derived(Data)]
pub struct PluginInfo(pub String, pub Option<String>);

/// An eval's ask of the MCP, the host of every worker it opens.
#[cereal::derived(Data)]
pub enum SessionAsk {
    /// A body's packet for the MCP's Channel.
    ChannelSend(ChannelSendData),
    /// The stop a spam verdict made, for the error's emergency.
    Stopped(ChannelSpamErrorEmergency),
    /// A body's message for a peer the MCP links, its files read on the worker.
    RemoteSend(PreparedSend),
    /// The MCP's configuration in force.
    Config,
}

/// A remote send as its sender prepares it: the event's NUON and each file's bytes.
#[cereal::derived(Data)]
pub struct PreparedSend {
    pub mcp_nom: String,
    pub model: String,
    /// The event as NUON text.
    pub event_nuon: String,
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
