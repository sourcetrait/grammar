use crate::*;

impl ChannelFace {
    /// `interface remote channel listen`: bind for a peer, blocking on the bind.
    pub(crate) async fn remote_listen(
        &self,
        nonce: datum::Nonce,
        p: RemoteListenParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        if let Err(error) = self.remote_gate().await {
            return error_to_call_result(error, nonce);
        }
        let listen = RemoteListenRequest {
            alias: p.alias,
            on: p.on,
        };
        self.remote_answer(nonce, listen).await
    }
}
