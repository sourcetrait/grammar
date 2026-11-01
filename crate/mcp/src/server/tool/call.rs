use crate::*;

/// Parameters for `call()`.
#[derive(Debug, ser::Deserialize, ser::Serialize, schema::JsonSchema)]
pub struct CallParams {
    /// The host alias the tool runs on; `local` for this MCP's own host.
    pub host: String,
    /// The function's namepath: `rig:module/path:function`.
    pub namepath: String,
    /// The function's data as NUON text; `null` for `data: nothing`.
    pub data: String,
    /// The call's timeout as a Nu duration, such as `30sec`; 120sec by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
}

/// The tool's description.
pub(crate) const CALL_DESCRIPTION: &str = "Invoke a committed rig function on its typed data.";

#[mcp::tool_router(router = call_router, vis = "pub(crate)")]
impl NuSh {
    #[doc = include_str!("../../../../../include/mcp/tool/call.md")]
    #[mcp::tool(description = CALL_DESCRIPTION)]
    pub(crate) async fn call(
        &self,
        mcp::Parameters(p): mcp::Parameters<CallParams>,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let nonce = self.mint("call", &p).to_nonce();
        let route = match self.route(&p.host) {
            Ok(route) => route,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let request = CallRequest {
            host: Host::Local,
            nonce,
            namepath: p.namepath,
            data: p.data,
            timeout: p.timeout,
        };
        match self.value(route, request).await {
            Ok(value) => success_value(nonce, value),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
