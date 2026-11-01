use crate::*;

impl NuSh {
    /// `interface remote engine import`: an entry set from an ssh config's host.
    pub(crate) async fn engine_import(
        &self,
        nonce: datum::Nonce,
        p: RemoteEngineImportParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let import = async {
            check_alias(&p.alias).map_err(engine_invalid)?;
            let from = p.from.clone().unwrap_or_else(|| p.alias.clone());
            let ssh_config = p.ssh_config.clone();
            let change = tk::spawn_blocking(move || import_ssh(&from, ssh_config.as_deref()))
                .await
                .map_err(|e| engine_invalid(format!("the import's task failed: {e}")))?
                .map_err(engine_invalid)?;
            self.write_engine(&p.alias, &change).await
        };
        match import.await {
            Ok(row) => success(nonce, &row),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
