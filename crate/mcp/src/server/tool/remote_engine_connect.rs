use crate::*;

impl NuSh {
    /// `interface remote engine connect`: the session and its worker started now.
    pub(crate) async fn engine_connect(
        &self,
        nonce: datum::Nonce,
        p: RemoteAliasParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let connect = async {
            let entry = self
                .engine_entries()?
                .into_iter()
                .find(|entry| entry.alias == p.alias)
                .ok_or_else(|| engine_invalid(format!("no remote engine `{}` in the remotes file", p.alias)))?;
            let endpoint = entry.endpoint().map_err(engine_invalid)?;
            self.controller.remote_connect(endpoint).await?;
            self.engine_row(&p.alias).await
        };
        match connect.await {
            Ok(row) => success(nonce, &row),
            Err(error) => error_to_call_result(error, nonce),
        }
    }

    /// `interface remote engine close`: the session closed, its worker ending with it.
    pub(crate) async fn engine_close(
        &self,
        nonce: datum::Nonce,
        p: RemoteAliasParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        if self.controller.remote_close(&p.alias).await {
            success_void(nonce)
        } else {
            error_to_call_result(GrammarMcpError::RemoteEngineNotOpen { alias: p.alias }, nonce)
        }
    }
}
