use crate::*;

/// The tool's name on the grammar server.
pub(crate) const BED_TOOL: &str = "bed";

/// User's pretext, verbatim, opening the tool's description.
const BED_PRETEXT: &str = include_str!("../../../../../include/human/mcp/bed.md");

/// Parameters for `bed()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct BedParams {
    /// The host alias the tool runs on; `local` for this MCP's own host.
    pub host: String,
    /// One embedded engine command: `grmr`.
    pub command: String,
    /// Its subcommand, its arguments literals.
    pub subcommand: String,
}

/// The tool's description: its pretext, then the `grmr` signatures deny leaves.
pub(crate) fn bed_description(deny: &DenySet) -> String {
    let signatures = grmr_signatures_where(|name| deny.denies_embedded(name));
    format!("{}\n\n{}", BED_PRETEXT.trim_end(), signatures.trim_end())
}

/// The tool's stub description; the router carries `bed_description`.
pub(crate) const BED_DESCRIPTION: &str = "Runs an embedded Grammar engine command directly.";

#[mcp::tool_router(router = bed_router, vis = "pub(crate)")]
impl NuSh {
    #[mcp::tool(description = BED_DESCRIPTION)]
    pub(crate) async fn bed(
        &self,
        mcp::Parameters(p): mcp::Parameters<BedParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint(BED_TOOL, &p).to_nonce();
        let route = match self.route(&p.host) {
            Ok(route) => route,
            Err(error) => return error_to_call_result(error, nonce),
        };
        if let Some(name) = withheld(&self.controller.config.deny, &p.command, &p.subcommand) {
            let row = Diagnostic::error(KIND_BED, None, format!("`{name}` is withheld here"));
            return error_to_call_result(EngineError::BedViolations { diagnostics: vec![row] }, nonce);
        }
        let request = BedRequest {
            host: Host::Local,
            nonce,
            command: p.command,
            subcommand: p.subcommand,
        };
        match self.value(route, request).await {
            Ok(value) => success_value(nonce, value),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}

/// The denied `grmr` command a bed call names, by the longest name it opens with.
fn withheld(
    deny: &DenySet,
    command: &str,
    subcommand: &str,
) -> Option<&'static str> {
    let (text, _) = with_head(subcommand, command.trim());
    let text = text.trim_start();
    GRMR_COMMANDS
        .iter()
        .copied()
        .filter(|name| {
            text.strip_prefix(name)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
        })
        .max_by_key(|name| name.len())
        .filter(|name| deny.denies_embedded(name))
}
