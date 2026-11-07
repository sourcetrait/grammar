use crate::*;

/// Parameters for `await()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct AwaitParams {
    /// The host alias the tool runs on; `local` for this MCP's own host.
    pub host: String,
    /// One Nu module exporting `prepare`, `execute`, `status`, and `stop`.
    pub module: String,
    /// prepare's data as NUON text; `null` for `data: nothing`.
    pub data: String,
    /// Each definition's timeout as a NUON record of durations or nulls; 120sec each by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<String>,
    /// The gaps between status beats as a NUON list of durations, the last repeating.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heartbeats: Option<String>,
}

#[mcp::tool_router(router = await_router, vis = "pub(crate)")]
impl NuSh {
    #[allow(clippy::doc_overindented_list_items)]
    #[doc = include_str!("../../../../../include/mcp/tool/await.md")]
    #[mcp::tool(name = "await")]
    pub(crate) async fn await_suite(
        &self,
        mcp::Parameters(p): mcp::Parameters<AwaitParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("await", &p).to_nonce();
        let route = match self.route(&p.host) {
            Ok(route) => route,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let data = match nuon_data(&p.data, "prepare") {
            Ok(data) => data,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let timeouts = match nuon_timeouts(p.timeouts.as_deref()) {
            Ok(timeouts) => timeouts,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let heartbeats = match nuon_heartbeats(p.heartbeats.as_deref()) {
            Ok(heartbeats) => heartbeats,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let request = AwaitRequest {
            host: Host::Local,
            nonce,
            module: p.module,
            data,
            timeouts,
            heartbeats,
        };
        match self.value(route, request).await {
            Ok(value) => success_value(nonce, value),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
