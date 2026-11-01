use crate::*;

impl ChannelFace {
    /// `interface remote channel set`: make or change an entry, answering its row.
    pub(crate) async fn remote_set(
        &self,
        nonce: datum::Nonce,
        p: RemoteSetParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let set = RemoteSetRequest {
            alias: p.alias,
            address: p.address,
            listen: p.listen,
            cert: p.cert,
        };
        self.remote_answer(nonce, set).await
    }

    /// `interface remote channel unset`: remove a remote's entry; a live link stays.
    pub(crate) async fn remote_unset(
        &self,
        nonce: datum::Nonce,
        p: RemoteAliasParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        self.remote_answer(nonce, RemoteUnsetRequest { alias: p.alias }).await
    }
}
