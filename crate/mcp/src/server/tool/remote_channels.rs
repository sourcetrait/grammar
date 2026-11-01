use crate::*;

impl ChannelFace {
    /// `interface remote channel list`: every remote, configured or live, by alias.
    pub(crate) async fn remote_list(
        &self,
        nonce: datum::Nonce,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        self.remote_answer(nonce, RemoteListRequest).await
    }

    /// `interface remote channel get`: one remote's entry and state.
    pub(crate) async fn remote_get(
        &self,
        nonce: datum::Nonce,
        p: RemoteAliasParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        self.remote_answer(nonce, RemoteGetRequest { alias: p.alias }).await
    }

    /// A remote command's answer from the peer links, as its tool renders it.
    pub(crate) async fn remote_answer<T>(
        &self,
        nonce: datum::Nonce,
        req: T,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData>
    where
        T: subsys::Request<PeerSystem, ResponseType = Result<RemoteResult, RuntimeError>>,
    {
        match self.peer.request(req).await {
            Ok(Ok(RemoteResult::Rows(rows))) => success(nonce, &rows),
            Ok(Ok(RemoteResult::Row(row))) => success(nonce, &row),
            Ok(Ok(RemoteResult::Listening(listening))) => success(nonce, &listening),
            Ok(Ok(RemoteResult::Done)) => success_void(nonce),
            Ok(Err(error)) => error_to_call_result(GrammarMcpError::from(error), nonce),
            Err(reason) => error_to_call_result(peer_failed(reason), nonce),
        }
    }

    /// A listen's or a connect's gate: a verified Channel for the link to report on.
    pub(crate) async fn remote_gate(&self) -> GrammarMcpResult<()> {
        if !matches!(self.status().await?.state.phase, ChannelPhase::Verified) {
            return Err(GrammarMcpError::RemoteChannelUnverified);
        }
        Ok(())
    }
}
