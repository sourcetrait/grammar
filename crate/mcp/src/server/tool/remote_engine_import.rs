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
            let imported = tk::spawn_blocking(move || import_ssh(&from, ssh_config.as_deref()))
                .await
                .map_err(|e| engine_invalid(format!("the import's task failed: {e}")))?
                .map_err(engine_invalid)?;
            let dir = EngineDefaults::of(&self.controller.config).dir;
            let file = write_known_hosts(&dir, &p.alias, &imported.name, &imported.keys).map_err(engine_invalid)?;
            let change = EngineChange {
                known_hosts: Some(KnownHosts::File(file)),
                ..imported.change
            };
            self.write_engine(&p.alias, &change).await
        };
        match import.await {
            Ok(row) => success(nonce, &row),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
