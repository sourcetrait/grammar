use crate::*;

/// `interface remote engine connect`'s answer: the opened engine's nom.
#[derive(Debug, ser::Serialize)]
struct EngineConnected {
    nom: String,
}

impl NuSh {
    /// `interface remote engine connect`: the session and its grammar-ssh started now.
    pub(crate) async fn engine_connect(
        &self,
        nonce: datum::Nonce,
        p: RemoteAliasParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let connect = async {
            let entries = self.engine_entries()?;
            let entry = entries
                .iter()
                .find(|entry| entry.alias == p.alias)
                .ok_or_else(|| engine_invalid(format!("no remote engine `{}` in the remotes file", p.alias)))?;
            let endpoint = entry
                .endpoint(&entries, &EngineDefaults::of(&self.controller.config))
                .map_err(engine_invalid)?;
            let state = self.controller.remote_connect(endpoint).await?;
            state.nom.ok_or_else(|| GrammarMcpError::Internal {
                phase: "connect".to_string(),
                reason: format!("remote engine `{}`'s open session names no nom", p.alias),
            })
        };
        match connect.await {
            Ok(nom) => success(nonce, &EngineConnected { nom }),
            Err(error) => error_to_call_result(error, nonce),
        }
    }

    /// `interface remote engine close`: the session closed, grammar-ssh ending with it.
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
