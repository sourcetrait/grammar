use crate::*;

/// User's envelope schema, heading the server's instructions.
const MCP_DOC: &str = include_str!("../../../../../include/human/mcp/mcp.md");

impl NuSh {
    /// The six tools the deny set leaves, `interface`'s and `bed`'s descriptions set.
    pub(crate) fn tool_router(deny: &DenySet) -> mcp::ToolRouter<Self> {
        let mut router = mcp::ToolRouter::new();
        if !deny.denies(DeniableTool::Execute) {
            router += Self::execute_router();
        }
        if !deny.denies(DeniableTool::Await) {
            router += Self::await_router();
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
        if !deny.denies(DeniableTool::Bed) {
            let mut bed = Self::bed_router();
            if let Some(route) = bed.map.get_mut(BED_TOOL) {
                route.attr.description = Some(bed_description(deny).into());
            }
            router += bed;
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
                 (the rerun(nonce) handle); an await call's suite, its trace of events, \
                 and each definition's logs at \
                 {cache}/await/<nonce>/{{suite.nu,trace.nuonl,<definition>/}}. \
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
