use crate::*;

/// The tool's name on the grammar server.
pub(crate) const INTERFACE_TOOL: &str = "interface";

/// User's pretext, verbatim, opening the tool's description.
const INTERFACE_PRETEXT: &str = include_str!("../../../../../include/human/mcp/interface.md");

/// Parameters for `interface()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct InterfaceParams {
    /// The host alias the tool runs on; `local` for this MCP's own host.
    pub host: String,
    /// One `interface` subcommand, its arguments literals.
    pub subcommand: String,
}

/// What a request says of its caller: its client's name and the thread it names.
#[derive(Debug, Clone, Default)]
pub(crate) struct Caller {
    /// The client's `clientInfo` name, the request's own or its `initialize`'s.
    pub(crate) client: Option<String>,
    /// The thread the request's `_meta` names, Codex's `threadId`.
    pub(crate) thread: Option<String>,
}

impl Caller {
    /// The caller a tool call's context describes.
    pub(crate) fn of(context: &mcp::RequestContext<mcp::RoleServer>) -> Self {
        Self::named(context.client_info().map(|info| info.name), &context.meta)
    }

    /// The caller a client's name and a request's `_meta` describe.
    pub(crate) fn named(
        client: Option<String>,
        meta: &mcp::RequestMetaObject,
    ) -> Self {
        let thread = meta
            .get(THREAD_ID_META)
            .and_then(|thread| thread.as_str())
            .filter(|thread| !thread.is_empty())
            .map(str::to_string);
        Self { client, thread }
    }
}

/// The tool's description: the pretext, then the commands the deny set leaves.
pub(crate) fn interface_description(deny: &DenySet) -> String {
    format!("{}\n\n{}", INTERFACE_PRETEXT.trim_end(), render_command_list(deny))
}

/// The tool's stub description; the router carries `interface_description`.
pub(crate) const INTERFACE_DESCRIPTION: &str = "Interfaces directly with the Grammar MCP.";

/// The parse's engine, holding the commands the deny set leaves.
pub(crate) fn interface_engine_for(deny: &DenySet) -> nu::EngineState {
    sourcetrait_grammar_interface::interface_engine(|words| deny.denies_command(words))
}

/// Whether `command` asks for one of the MCP's own blocks, the interface's or grmr's.
fn mcp_block(command: &EngineInterfaceCommand) -> bool {
    matches!(command, EngineInterfaceCommand::Help(p) if p.namepath == INTERFACE || p.namepath == GRMR)
}

/// The commands the deny set leaves, by their words, a `##` heading a topic.
pub(crate) fn render_command_list(deny: &DenySet) -> String {
    sourcetrait_grammar_protocol::render_command_list(|words| deny.denies_command(words))
}

/// The signatures the deny set leaves, a `###` heading a topic.
pub(crate) fn render_signatures(deny: &DenySet) -> String {
    sourcetrait_grammar_protocol::render_signatures(|words| deny.denies_command(words))
}

impl ChannelFace {
    /// A Channel command, routed to the Channel's system through the face.
    pub(crate) async fn run_channel(
        &self,
        nonce: datum::Nonce,
        command: ChannelCommand,
        caller: &Caller,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match command {
            ChannelCommand::Open => self.channel_open(nonce, caller).await,
            ChannelCommand::Sync => self.channel_sync(nonce).await,
            ChannelCommand::Ready => self.channel_ready(nonce).await,
            ChannelCommand::Close => self.channel_close(nonce).await,
            ChannelCommand::Config(p) => self.channel_config(nonce, p).await,
        }
    }

    /// A remote channel command, routed to the peer links' system through the face.
    pub(crate) async fn run_peer(
        &self,
        nonce: datum::Nonce,
        command: PeerCommand,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match command {
            PeerCommand::List => self.remote_list(nonce).await,
            PeerCommand::Get(p) => self.remote_get(nonce, p).await,
            PeerCommand::Set(p) => self.remote_set(nonce, p).await,
            PeerCommand::Unset(p) => self.remote_unset(nonce, p).await,
            PeerCommand::Listen(p) => self.remote_listen(nonce, p).await,
            PeerCommand::Connect(p) => self.remote_connect(nonce, p).await,
            PeerCommand::Close(p) => self.remote_close(nonce, p).await,
        }
    }

    /// A chat command, sent over the peer links or read from the Channel's inbox.
    pub(crate) async fn run_chat(
        &self,
        nonce: datum::Nonce,
        command: ChatCommand,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match command {
            ChatCommand::Send(p) => self.chat_send(nonce, p).await,
            ChatCommand::Read(p) => self.chat_read(nonce, p).await,
        }
    }
}

#[mcp::tool_router(router = interface_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(description = INTERFACE_DESCRIPTION)]
    pub(crate) async fn interface(
        &self,
        context: mcp::RequestContext<mcp::RoleServer>,
        mcp::Parameters(p): mcp::Parameters<InterfaceParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        self.interface_as(Caller::of(&context), p).await
    }
}

impl NuSh {
    /// The `interface` tool for a caller: the parse, then the command its system runs.
    pub(crate) async fn interface_as(
        &self,
        caller: Caller,
        p: InterfaceParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint(INTERFACE_TOOL, &p).to_nonce();
        let route = match self.route(&p.host) {
            Ok(route) => route,
            Err(error) => return error_to_call_result(error, nonce),
        };
        match parse_command(&self.interface_engine, &p.subcommand) {
            Ok(command) => {
                if matches!(command, InterfaceCommand::Engine(EngineInterfaceCommand::Info)) {
                    self.controller.face.keep_thread(caller.thread.as_deref());
                }
                self.run_interface(nonce, route, command, &p.subcommand, &caller).await
            }
            Err(diagnostics) => {
                error_to_call_result(GrammarMcpError::InterfaceViolations { diagnostics }, nonce)
            }
        }
    }

    /// Run a parsed command under the interface call's nonce, routed to its system.
    ///
    /// An engine command bound for a remote engine goes as `subcommand`, its text.
    pub(crate) async fn run_interface(
        &self,
        nonce: datum::Nonce,
        route: Route,
        command: InterfaceCommand,
        subcommand: &str,
        caller: &Caller,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let face = &self.controller.face;
        match command {
            InterfaceCommand::Channel(command) => face.run_channel(nonce, command, caller).await,
            InterfaceCommand::Peer(command) => face.run_peer(nonce, command).await,
            InterfaceCommand::Chat(command) => face.run_chat(nonce, command).await,
            InterfaceCommand::RemoteEngine(command) => self.run_remote_engine(nonce, command).await,
            InterfaceCommand::Engine(command) => match route {
                Route::Remote(alias) if !mcp_block(&command) => {
                    self.remote_interface(nonce, &alias, subcommand).await
                }
                _ => self.local_engine(nonce, command).await,
            },
        }
    }

    /// A remote engine command, run on the MCP whatever the tool's host.
    async fn run_remote_engine(
        &self,
        nonce: datum::Nonce,
        command: RemoteEngineCommand,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match command {
            RemoteEngineCommand::List => self.engine_list(nonce).await,
            RemoteEngineCommand::Get(p) => self.engine_get(nonce, p).await,
            RemoteEngineCommand::Set(p) => self.engine_set(nonce, *p).await,
            RemoteEngineCommand::Unset(p) => self.engine_unset(nonce, p).await,
            RemoteEngineCommand::Connect(p) => self.engine_connect(nonce, p).await,
            RemoteEngineCommand::Close(p) => self.engine_close(nonce, p).await,
            RemoteEngineCommand::Import(p) => self.engine_import(nonce, p).await,
        }
    }

    /// An engine command on the MCP's own engine, within the session's view there.
    pub(crate) async fn local_engine(
        &self,
        nonce: datum::Nonce,
        command: EngineInterfaceCommand,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        answer_result(nonce, run_engine_command(&LocalDesk(self), command).await)
    }

    /// An interface call sent to a remote engine as its subcommand, run there.
    async fn remote_interface(
        &self,
        nonce: datum::Nonce,
        alias: &str,
        subcommand: &str,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let request = InterfaceRequest {
            nonce,
            subcommand: subcommand.to_string(),
        };
        match self.controller.remote(alias, request.into()).await {
            Ok(EngineResponse::Answer(answer)) => answer_result(nonce, answer.result),
            Ok(EngineResponse::Refused(refusal)) | Err(refusal) => {
                error_to_call_result(GrammarMcpError::Transport(refusal), nonce)
            }
        }
    }
}
