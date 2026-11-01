use crate::*;

/// The server: the four tools on the in-process engine.
pub struct NuSh {
    /// mcp's controller: the engine's requests and the face's commands go to it.
    pub(crate) controller: Arc<McpController>,
    pub(crate) nonce_gen: Arc<datum::NonceGenerator>,
    /// This host process's identity, minted once at startup.
    pub(crate) mcp_nom: datum::NomPair,
    /// The engine an interface command parses on: its commands, nothing else.
    pub(crate) interface_engine: Arc<nu::EngineState>,
    /// Which purview ids this session has in view on each engine.
    pub(crate) views: Arc<Views>,
    pub(crate) tool_router: mcp::ToolRouter<NuSh>,
}

impl NuSh {
    /// The server over `controller`, its interface engine denying `deny`'s commands.
    pub(crate) fn new(
        controller: Arc<McpController>,
        deny: &DenySet,
    ) -> Self {
        Self {
            nonce_gen: controller.face.nonce_gen.clone(),
            mcp_nom: controller.face.mcp_nom.clone(),
            interface_engine: Arc::new(interface_engine_for(deny)),
            views: Arc::new(Views::default()),
            tool_router: Self::tool_router(&controller.config.deny),
            controller,
        }
    }

    /// A call's nonce, minted first from its tool and its parameters.
    pub(crate) fn mint<P: ser::Serialize>(
        &self,
        tool: &str,
        params: &P,
    ) -> datum::NoncePair {
        mint_nonce(&self.nonce_gen, tool, params)
    }

    /// A control command's answer from the engine `route` names.
    pub(crate) async fn control_on(
        &self,
        route: &Route,
        command: EngineCommand,
    ) -> GrammarMcpResult<ControlAnswer> {
        match route {
            Route::Local => Ok(self.controller.control(command).await?),
            Route::Remote(alias) => {
                let request = EngineRequest::Wire(ToEngineSys::Control(ControlRequest {
                    host: Host::Local,
                    command,
                }));
                match self.controller.remote(alias, request).await? {
                    EngineResponse::Wire(FromEngineSys::Control(answer)) => Ok(answer?),
                    EngineResponse::Refused(refusal) => Err(GrammarMcpError::Transport(refusal)),
                    _ => Err(unexpected("control")),
                }
            }
        }
    }

    /// A remote engine's worker: its identity, its homes, and its plugins.
    pub(crate) async fn worker_info(
        &self,
        alias: &str,
    ) -> GrammarMcpResult<WorkerInfo> {
        match self.controller.remote(alias, EngineRequest::Info).await? {
            EngineResponse::Info(info) => Ok(info),
            EngineResponse::Refused(refusal) => Err(GrammarMcpError::Transport(refusal)),
            _ => Err(unexpected("info")),
        }
    }

    /// The signature block within pattern strings, as the routed engine renders it.
    pub(crate) async fn signatures(
        &self,
        route: &Route,
        patterns: Vec<String>,
    ) -> GrammarMcpResult<String> {
        match self.control_on(route, EngineCommand::Signatures { patterns }).await? {
            ControlAnswer::Signatures(block) => Ok(block),
            _ => Err(unexpected("signatures")),
        }
    }
}

/// Where a tool's `host` sends its request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Route {
    /// The MCP's own engine.
    Local,
    /// The configured remote engine of this alias.
    Remote(String),
}

impl NuSh {
    /// A tool's `host` as its route: `local`, a whole remote engine entry, or unknown.
    pub(crate) fn route(&self, alias: &str) -> GrammarMcpResult<Route> {
        if alias == LOCAL_HOST {
            return Ok(Route::Local);
        }
        let entries = read_engines(&self.controller.config.remotes_file)
            .map_err(|reason| GrammarMcpError::RemotesFile { reason })?;
        match entries.into_iter().find(|entry| entry.alias == alias) {
            Some(entry) => {
                entry
                    .endpoint(&EngineDefaults::of(&self.controller.config))
                    .map_err(|reason| GrammarMcpError::RemoteEngineInvalidParams { reason })?;
                Ok(Route::Remote(alias.to_string()))
            }
            None => Err(GrammarMcpError::HostUnknown {
                host: alias.to_string(),
            }),
        }
    }

    /// An eval's, a rerun's, or a call's value from the engine its route names.
    pub(crate) async fn value<T>(
        &self,
        route: Route,
        request: T,
    ) -> GrammarMcpResult<nu::Value>
    where
        T: subsys::Request<EngineSystem, ResponseType = ValueAnswer> + Into<ToEngineSys>,
    {
        let answer = match route {
            Route::Local => self.controller.request(request).await,
            Route::Remote(alias) => {
                let response = self.controller.remote(&alias, EngineRequest::Wire(request.into())).await;
                remote_value(response)?
            }
        };
        answer_value(answer)
    }
}

/// A remote engine's response as its value answer, or its transport's error.
fn remote_value(
    response: Result<EngineResponse, TransportError>,
) -> GrammarMcpResult<EngineResult<ValueAnswer>> {
    match response? {
        EngineResponse::Wire(FromEngineSys::Value(answer)) => Ok(Ok(answer)),
        EngineResponse::Refused(refusal) => Err(GrammarMcpError::Transport(refusal)),
        _ => Err(unexpected("value")),
    }
}

/// The internal error an answer of another command's kind is.
pub(crate) fn unexpected(command: &str) -> GrammarMcpError {
    GrammarMcpError::Internal {
        phase: format!("engine::{command}"),
        reason: "the engine answered another command".to_string(),
    }
}

/// An envelope, `nonce` first and one more field, as the result's one NUON text.
pub(crate) fn envelope_text(
    nonce: datum::Nonce,
    field: &str,
    value: nu::Value,
) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    let span = nu::Span::unknown();
    let mut envelope = nu::Record::new();
    envelope.insert("nonce", nu::Value::string(nonce.to_string(), span));
    envelope.insert(field, value);
    let text = render_nuon(&nu::Value::record(envelope, span))
        .map_err(|e| mcp::ErrorData::internal_error(format!("envelope render: {e}"), None))?;
    Ok(mcp::CallToolResult::success(vec![mcp::ContentBlock::text(text)]))
}

/// A tool's record as a nu value, written by serde as a NuVal through nuval.
pub(crate) fn serde_to_nu<T: ser::Serialize>(value: &T) -> Result<nu::Value, mcp::ErrorData> {
    nuval::to_nuval(value)
        .map_err(|e| mcp::ErrorData::internal_error(format!("envelope serialize: {e}"), None))?
        .try_into_nu(nu::Span::unknown())
        .map_err(|e| mcp::ErrorData::internal_error(format!("envelope convert: {e}"), None))
}

/// A success, `{nonce, result}`: the result a nu value the tool built.
pub(crate) fn success_value(
    nonce: datum::Nonce,
    result: nu::Value,
) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    envelope_text(nonce, "result", result)
}

/// A success, `{nonce, result}`: the result a tool's record.
pub(crate) fn success<T: ser::Serialize>(
    nonce: datum::Nonce,
    result: &T,
) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    success_value(nonce, serde_to_nu(result)?)
}

/// A success with nothing to answer, `{nonce, result: null}`.
pub(crate) fn success_void(nonce: datum::Nonce) -> Result<mcp::CallToolResult, mcp::ErrorData> {
    success_value(nonce, nu::Value::nothing(nu::Span::unknown()))
}

/// A call's value from the engine, its nushell form, or why it has none.
pub(crate) fn answer_value(answer: EngineResult<ValueAnswer>) -> GrammarMcpResult<nu::Value> {
    let value = answer?.result?;
    value
        .try_into_nu(nu::Span::unknown())
        .map_err(|e| GrammarMcpError::Internal {
            phase: "engine::value".to_string(),
            reason: e.to_string(),
        })
}

pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
