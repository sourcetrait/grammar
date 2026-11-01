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
    /// One `interface` command, its arguments literals.
    pub command: String,
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
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match command {
            ChannelCommand::Open(p) => self.channel_open(nonce, p).await,
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
}

#[mcp::tool_router(router = interface_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(description = INTERFACE_DESCRIPTION)]
    pub(crate) async fn interface(
        &self,
        mcp::Parameters(p): mcp::Parameters<InterfaceParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint(INTERFACE_TOOL, &p).to_nonce();
        let route = match self.route(&p.host) {
            Ok(route) => route,
            Err(error) => return error_to_call_result(error, nonce),
        };
        match parse_command(&self.interface_engine, &p.command) {
            Ok(command) => self.run_interface(nonce, route, command).await,
            Err(diagnostics) => {
                error_to_call_result(GrammarMcpError::InterfaceViolations { diagnostics }, nonce)
            }
        }
    }
}

impl NuSh {
    /// Run a parsed command under the interface call's nonce, routed to its system.
    pub(crate) async fn run_interface(
        &self,
        nonce: datum::Nonce,
        route: Route,
        command: InterfaceCommand,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let face = &self.controller.face;
        match command {
            InterfaceCommand::Channel(command) => face.run_channel(nonce, command).await,
            InterfaceCommand::Peer(command) => face.run_peer(nonce, command).await,
            InterfaceCommand::Engine(command) => self.run_engine(nonce, &route, command).await,
        }
    }

    /// An engine command, routed to its host's engine with the session's view there.
    async fn run_engine(
        &self,
        nonce: datum::Nonce,
        route: &Route,
        command: EngineInterfaceCommand,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        use EngineInterfaceCommand as E;
        match command {
            E::Info => self.info(nonce, route).await,
            E::Help(p) => self.help(nonce, route, p).await,
            E::ProcessList => self.process_list(nonce, route).await,
            E::ProcessKill(p) => self.process_kill(nonce, route, p).await,
            E::PurviewList => self.purview_list(nonce, route).await,
            E::PurviewSet(p) => self.purview_set(nonce, route, p).await,
            E::PurviewExtend(p) => self.purview_extend(nonce, route, p).await,
            E::PurviewConfigure(p) => self.purview_configure(nonce, route, p).await,
            E::EquipNew(p) => self.equip_new(nonce, route, p).await,
            E::EquipInstall(p) => self.equip_install(nonce, route, p).await,
            E::EquipCheck(p) => self.equip_check(nonce, route, p).await,
            E::EquipUninstall(p) => self.equip_uninstall(nonce, route, p).await,
            E::EquipScaffold(p) => self.equip_scaffold(nonce, route, p).await,
            E::EquipCommit(p) => self.equip_commit(nonce, route, p).await,
        }
    }
}
