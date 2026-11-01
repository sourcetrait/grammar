use crate::*;

impl NuSh {
    /// `interface remote engine set`: each field given checked, then written.
    pub(crate) async fn engine_set(
        &self,
        nonce: datum::Nonce,
        p: RemoteEngineSetParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let RemoteEngineSetParams {
            alias,
            address,
            user,
            key,
            known_hosts,
            known_hosts_file,
            command,
            id,
            namespace,
            workdir,
        } = p;
        let known_hosts = match (known_hosts, known_hosts_file) {
            (Some(_), Some(_)) => {
                let refused = engine_invalid("give `--known-hosts` or `--known-hosts-file`, not both".to_string());
                return error_to_call_result(refused, nonce);
            }
            (Some(keys), None) => Some(KnownHosts::Keys(keys)),
            (None, Some(file)) => Some(KnownHosts::File(file)),
            (None, None) => None,
        };
        let change = EngineChange {
            address,
            user,
            key,
            known_hosts,
            command,
            id,
            namespace,
            workdir,
        };
        match self.write_engine(&alias, &change).await {
            Ok(row) => success(nonce, &row),
            Err(error) => error_to_call_result(error, nonce),
        }
    }

    /// `change` checked, written to `alias`'s entry, and the entry's row read back.
    pub(crate) async fn write_engine(
        &self,
        alias: &str,
        change: &EngineChange,
    ) -> GrammarMcpResult<RemoteEngineRow> {
        EngineEntry::of(alias, change).check_given().map_err(engine_invalid)?;
        edit_remotes(&self.controller.config.remotes_file, |text| set_engine(text, alias, change))
            .map_err(|reason| GrammarMcpError::RemotesFile { reason })?;
        self.engine_row(alias).await
    }

    /// `interface remote engine unset`: the entry removed, a live session left open.
    pub(crate) async fn engine_unset(
        &self,
        nonce: datum::Nonce,
        p: RemoteAliasParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let unset = || -> GrammarMcpResult<()> {
            if !self.engine_entries()?.iter().any(|entry| entry.alias == p.alias) {
                return Err(engine_invalid(format!("no remote engine `{}` in the remotes file", p.alias)));
            }
            edit_remotes(&self.controller.config.remotes_file, |text| unset_engine(text, &p.alias))
                .map_err(|reason| GrammarMcpError::RemotesFile { reason })
        };
        match unset() {
            Ok(()) => success_void(nonce),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
