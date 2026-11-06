use crate::*;

/// `grmr remote channel send <mcp_nom> <model> <data> [attached]` - a message.
#[derive(Clone)]
pub(crate) struct GrmrRemoteChannelSend {
    pub(crate) host: Option<HostLink>,
}

impl GrmrRemoteChannelSend {
    pub(crate) const DEF_MCP_NOM: nuin::ParameterDef = nuin::ParameterDef {
        name: "mcp_nom",
        description: "Remote MCP",
    };
    pub(crate) const DEF_MODEL: nuin::ParameterDef = nuin::ParameterDef {
        name: "model",
        description: "Namepath of the data's shape",
    };
    pub(crate) const DEF_DATA: nuin::ParameterDef = nuin::ParameterDef {
        name: "data",
        description: "Notification data",
    };
    pub(crate) const DEF_ATTACHED: nuin::ParameterDef = nuin::ParameterDef {
        name: "attached",
        description: "Files to deliver, table<src: path, dest: path>: local src, receiver dest",
    };
}

impl nu::Command for GrmrRemoteChannelSend {
    fn name(&self) -> &str { Self::DEF.name }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build(Self::DEF.name)
            .vocab(&Self::DEF)
            .vocab_required(&Self::DEF_MCP_NOM, nu::SyntaxShape::String)
            .vocab_required(&Self::DEF_MODEL, nu::SyntaxShape::String)
            .vocab_required(&Self::DEF_DATA, data_shape())
            .vocab_optional(
                &Self::DEF_ATTACHED,
                nu::SyntaxShape::Table(nu::CollectionColumns::from(vec![])),
            )
            .input_output_types(vec![(nu::Type::Nothing, event_id_type())])
    }

    fn examples(&self) -> Vec<nu::Example<'_>> { Self::DEF.examples() }

    fn description(&self) -> &str { Self::DEF.description }

    fn run(
        &self,
        engine_state: &nu::EngineState,
        stack: &mut nu::Stack,
        call: &nu::Call<'_>,
        _input: nu::PipelineData,
    ) -> Result<nu::PipelineData, nu::ShellError> {
        let mcp_nom: String = call.req(engine_state, stack, 0)?;
        let model: String = call.req(engine_state, stack, 1)?;
        let data: nu::Value = call.req(engine_state, stack, 2)?;
        require_record_or_table(&data, call.head)?;
        let attached: Option<nu::Value> = call.opt(engine_state, stack, 3)?;
        let name = nu::Command::name(self);
        let host = asked_host(&self.host, name, call.head)?;
        let id = remote_send(host, mcp_nom, model, data, attached)
            .map_err(|message| shell_error(name, &message, call.head))?;
        Ok(nu::PipelineData::Value(event_id_record(id, call.head), None))
    }
}

/// A body's send to a linked peer, asked of the host; its id, or why not.
fn remote_send(
    host: &HostLink,
    mcp_nom: String,
    model: String,
    data: nu::Value,
    attached: Option<nu::Value>,
) -> Result<String, String> {
    if let Some(prefix) = reserved_prefix(&model) {
        return Err(format!(
            "`{prefix}` is reserved for host-originated models; \
             choose a model path outside it",
        ));
    }
    if model_too_long(&model) {
        return Err(model_too_long_reason());
    }
    let send = RemoteSendData {
        mcp_nom,
        model,
        data: sendable(data)?,
        attached: attached.map(sendable).transpose()?,
    };
    match host.ask(HostAsk::RemoteSend(send))? {
        HostAnswer::Sent(id) => Ok(id),
        HostAnswer::Refused(reason) => Err(reason),
        other => Err(unexpected(&other)),
    }
}

/// A refused send: the command titles it, and the host's reason labels it.
fn shell_error(
    command: &str,
    message: &str,
    span: nu::Span,
) -> nu::ShellError {
    nu::GenericError::new(
        format!("{command} failed"),
        message.to_string(),
        span,
    )
    .into()
}
