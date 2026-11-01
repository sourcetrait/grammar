use crate::*;

/// An engine's row in `remote engine list` and `get`: its entry and its session.
#[derive(Debug, ser::Serialize)]
pub(crate) struct RemoteEngineRow {
    pub alias: String,
    /// Whether the remotes file holds the alias's entry; a live session can outlast it.
    pub configured: bool,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    /// The private key file, as the file writes it.
    pub key: Option<String>,
    pub host_keys: Option<Vec<String>>,
    pub command: Option<String>,
    pub id: Option<String>,
    pub namespace: Option<String>,
    pub workdir: Option<String>,
    /// The fields a session needs that the entry lacks.
    pub missing: Vec<String>,
    pub state: SessionPhase,
    pub worker_nom: Option<String>,
    pub worker_pid: Option<u32>,
    /// The last open's failure, its kind first.
    pub failure: Option<String>,
}

impl RemoteEngineRow {
    /// A row of `alias` from its entry, if any, and its session's state, if any.
    fn of(
        alias: &str,
        entry: Option<&EngineEntry>,
        state: Option<&SessionState>,
    ) -> Self {
        let EngineEntry {
            host,
            port,
            user,
            key,
            host_keys,
            command,
            id,
            namespace,
            workdir,
            ..
        } = entry.cloned().unwrap_or_default();
        Self {
            alias: alias.to_string(),
            configured: entry.is_some(),
            host,
            port,
            user,
            key,
            host_keys,
            command,
            id,
            namespace,
            workdir,
            missing: entry.map(EngineEntry::missing).unwrap_or_default(),
            state: state.map_or(SessionPhase::Closed, |state| state.phase),
            worker_nom: state.and_then(|state| state.worker_nom.clone()),
            worker_pid: state.and_then(|state| state.worker_pid),
            failure: state.and_then(|state| state.failure.clone()),
        }
    }
}

/// A remote engine command's refusal of its parameters.
pub(crate) fn engine_invalid(reason: String) -> GrammarMcpError {
    GrammarMcpError::RemoteEngineInvalidParams { reason }
}

impl NuSh {
    /// The remotes file's engines; a malformed file is refused whole.
    pub(crate) fn engine_entries(&self) -> GrammarMcpResult<Vec<EngineEntry>> {
        read_engines(&self.controller.config.remotes_file).map_err(|reason| GrammarMcpError::RemotesFile { reason })
    }

    /// Every engine's row by alias: the file's entries and the live sessions it lacks.
    pub(crate) async fn engine_rows(&self) -> GrammarMcpResult<Vec<RemoteEngineRow>> {
        let entries = self.engine_entries()?;
        let states = self.controller.remote_states().await;
        let mut aliases: Vec<String> = entries
            .iter()
            .map(|entry| entry.alias.clone())
            .chain(
                states
                    .iter()
                    .filter(|state| state.phase != SessionPhase::Closed)
                    .map(|state| state.alias.clone()),
            )
            .collect();
        aliases.sort();
        aliases.dedup();
        Ok(aliases
            .iter()
            .map(|alias| {
                let entry = entries.iter().find(|entry| &entry.alias == alias);
                let state = states.iter().find(|state| &state.alias == alias);
                RemoteEngineRow::of(alias, entry, state)
            })
            .collect())
    }

    /// One engine's row; an alias neither configured nor live is refused.
    pub(crate) async fn engine_row(
        &self,
        alias: &str,
    ) -> GrammarMcpResult<RemoteEngineRow> {
        self.engine_rows()
            .await?
            .into_iter()
            .find(|row| row.alias == alias)
            .ok_or_else(|| engine_invalid(format!("no remote engine `{alias}`")))
    }

    /// `interface remote engine list`: every engine, configured or live, by alias.
    pub(crate) async fn engine_list(
        &self,
        nonce: datum::Nonce,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.engine_rows().await {
            Ok(rows) => success(nonce, &rows),
            Err(error) => error_to_call_result(error, nonce),
        }
    }

    /// `interface remote engine get`: one engine's entry and its session.
    pub(crate) async fn engine_get(
        &self,
        nonce: datum::Nonce,
        p: RemoteAliasParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.engine_row(&p.alias).await {
            Ok(row) => success(nonce, &row),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
