use crate::*;

/// `grimm remote channel send <mcp_nom> <model> <event>` - a message, no files.
#[derive(Clone)]
pub(crate) struct GrimmRemoteChannelSend {
    pub(crate) host: Option<HostLink>,
}

/// `grimm remote_channel_send_with`: a message to a linked peer, with its files.
#[derive(Clone)]
pub(crate) struct GrimmRemoteChannelSendWith {
    pub(crate) host: Option<HostLink>,
}

impl GrimmRemoteChannelSend {
    pub(crate) const DEF_MCP_NOM: nuin::ParameterDef = nuin::ParameterDef {
        name: "mcp_nom",
        description: "Remote MCP",
    };
    pub(crate) const DEF_MODEL: nuin::ParameterDef = nuin::ParameterDef {
        name: "model",
        description: "Namepath of the event data type",
    };
    pub(crate) const DEF_EVENT: nuin::ParameterDef = nuin::ParameterDef {
        name: "event",
        description: "Notification data",
    };
}

impl nu::Command for GrimmRemoteChannelSend {
    fn name(&self) -> &str { Self::DEF.name }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build(Self::DEF.name)
            .vocab(&Self::DEF)
            .vocab_required(&Self::DEF_MCP_NOM, nu::SyntaxShape::String)
            .vocab_required(&Self::DEF_MODEL, nu::SyntaxShape::String)
            .vocab_required(&Self::DEF_EVENT, data_shape())
            .input_output_types(vec![(nu::Type::Nothing, event_id_type())])
    }

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
        let event: nu::Value = call.req(engine_state, stack, 2)?;
        require_record_or_table(&event, call.head)?;
        let name = nu::Command::name(self);
        let host = asked_host(&self.host, name, call.head)?;
        let id = remote_send(host, mcp_nom, model, event, None)
            .map_err(|message| shell_error(name, &message, call.head))?;
        Ok(nu::PipelineData::Value(event_id_record(id, call.head), None))
    }
}

impl nu::Command for GrimmRemoteChannelSendWith {
    fn name(&self) -> &str {
        "grimm remote_channel_send_with"
    }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build("grimm remote_channel_send_with")
            .required("mcp_nom", nu::SyntaxShape::String, "the linked peer host's McpNom")
            .required("model", nu::SyntaxShape::String, "the shape the event carries")
            .required("event", data_shape(), "the state summary to deliver")
            .required(
                "attached",
                nu::SyntaxShape::Table(nu::CollectionColumns::from(vec![])),
                "the files table<src: path, dest: path>: local src -> receiver dest name",
            )
            .input_output_types(vec![(nu::Type::Nothing, event_id_type())])
            .category(nu::Category::Custom("grimm".to_string()))
    }

    fn description(&self) -> &str {
        "Deliver a message plus files to a linked remote host's agent; returns the message id."
    }

    fn run(
        &self,
        engine_state: &nu::EngineState,
        stack: &mut nu::Stack,
        call: &nu::Call<'_>,
        _input: nu::PipelineData,
    ) -> Result<nu::PipelineData, nu::ShellError> {
        let mcp_nom: String = call.req(engine_state, stack, 0)?;
        let model: String = call.req(engine_state, stack, 1)?;
        let event: nu::Value = call.req(engine_state, stack, 2)?;
        require_record_or_table(&event, call.head)?;
        let attached: nu::Value = call.req(engine_state, stack, 3)?;
        let name = nu::Command::name(self);
        let host = asked_host(&self.host, name, call.head)?;
        let id = remote_send(host, mcp_nom, model, event, Some(attached))
            .map_err(|message| shell_error(name, &message, call.head))?;
        Ok(nu::PipelineData::Value(event_id_record(id, call.head), None))
    }
}

/// A body's send to a linked peer, asked of the host; its id, or why not.
fn remote_send(
    host: &HostLink,
    mcp_nom: String,
    model: String,
    event: nu::Value,
    attached: Option<nu::Value>,
) -> Result<String, String> {
    if let Some(prefix) = reserved_prefix(&model) {
        return Err(format!(
            "`{prefix}` is reserved for host-originated models; \
             choose a model path outside it",
        ));
    }
    let data = RemoteSendData {
        mcp_nom,
        model,
        event: sendable(event)?,
        attached: attached.map(sendable).transpose()?,
    };
    match host.ask(HostAsk::RemoteSend(data))? {
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
