use crate::*;

impl ChannelFace {
    /// `interface remote channel connect`: dial a peer, blocking on the connect.
    pub(crate) async fn remote_connect(
        &self,
        nonce: datum::Nonce,
        p: RemoteConnectParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        if let Err(error) = self.remote_gate().await {
            return error_to_call_result(error, nonce);
        }
        let connect = RemoteConnectRequest {
            alias: p.alias,
            address: p.address,
        };
        self.remote_answer(nonce, connect).await
    }
}
