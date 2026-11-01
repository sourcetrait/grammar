use crate::*;

impl ChannelFace {
    /// `interface remote channel close`: close an open link or a waiting listener.
    pub(crate) async fn remote_close(
        &self,
        nonce: datum::Nonce,
        p: RemoteChannelCloseParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        self.remote_answer(nonce, RemoteCloseRequest { alias: p.alias }).await
    }
}
