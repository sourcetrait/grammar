use crate::*;

/// User's envelope schema, heading the server's instructions.
const MCP_DOC: &str = include_str!("../../../../../include/human/mcp/mcp.md");

impl NuSh {
    /// The four tools the deny set leaves, `interface`'s description its commands.
    pub(crate) fn tool_router(deny: &DenySet) -> mcp::ToolRouter<Self> {
        let mut router = mcp::ToolRouter::new();
        if !deny.denies(DeniableTool::Execute) {
            router += Self::execute_router();
        }
        if !deny.denies(DeniableTool::Rerun) {
            router += Self::rerun_router();
        }
        if !deny.denies(DeniableTool::Call) {
            router += Self::call_router();
        }
        if !deny.denies(DeniableTool::Interface) {
            let mut interface = Self::interface_router();
            if let Some(route) = interface.map.get_mut(INTERFACE_TOOL) {
                route.attr.description = Some(interface_description(deny).into());
            }
            router += interface;
        }
        router
    }
}

#[mcp::tool_handler(router = self.tool_router)]
impl mcp::ServerHandler for NuSh {
    fn get_info(&self) -> mcp::ServerConfig {
        let config = &self.controller.config;
        let id = &config.id;
        let namespace = &config.namespace;
        let title = if namespace == "default" {
            "grammar".to_string()
        } else {
            format!("grammar ({namespace})")
        };
        mcp::ServerConfig::new(mcp::ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                mcp::Implementation::new(consts::GRAMMAR, env!("CARGO_PKG_VERSION"))
                    .with_title(title),
            )
            .with_instructions(format!(
                "{MCP_DOC}\n\
                 Evaluation artifacts are cached at \
                 {cache}/{{execute,calls}}/<nonce>/{{stdout,stderr}}; \
                 an execute call's definition co-locates at \
                 {cache}/execute/<nonce>/definition.nu \
                 (the rerun(nonce) handle). \
                 Installed equips live in a signed git repo at \
                 {data}/equip/; signing keypair at \
                 {data}/keypair/. \
                 This server's namespace: id `{id}`, namespace `{namespace}`. \
                 A tool whose `host` names a remote engine runs on that host, \
                 and the paths in its results are that host's own.",
                cache = config.cache_home().display(),
                data = config.data_home().display(),
            ))
    }
}
