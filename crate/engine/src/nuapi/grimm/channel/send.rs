use crate::*;

/// `grimm channel send <model> <event> [attached]` - the state-update lane.
#[derive(Clone)]
pub(crate) struct GrimmChannelSend {
    pub(crate) host: Option<HostLink>,
}

impl nu::Command for GrimmChannelSend {
    fn name(&self) -> &str { Self::DEF.name }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build(Self::DEF.name)
            .vocab(&Self::DEF)
            .vocab_required(&Self::DEF_MODEL, nu::SyntaxShape::String)
            .vocab_required(&Self::DEF_EVENT, data_shape())
            .vocab_optional(&Self::DEF_ATTACHED, data_shape())
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
        let model: String = call.req(engine_state, stack, 0)?;
        let event: nu::Value = call.req(engine_state, stack, 1)?;
        require_record_or_table(&event, call.head)?;
        let attached: Option<nu::Value> = call.opt(engine_state, stack, 2)?;
        if let Some(value) = &attached {
            require_record_or_table(value, call.head)?;
        }
        let name = nu::Command::name(self);
        let host = asked_host(&self.host, name, call.head)?;
        let from = NuapiCall::current(engine_state, stack, call.head)?.origin();
        let refused = |message: String| shell_error(name, &message, call.head);
        let data = ChannelSendData {
            from: from.clone(),
            model,
            event: sendable(event).map_err(refused)?,
            attached: attached.map(sendable).transpose().map_err(refused)?,
        };
        let id = match host.ask(HostAsk::ChannelSend(data)).map_err(refused)? {
            HostAnswer::Sent(id) => id,
            HostAnswer::Refused(reason) => return Err(refused(reason)),
            HostAnswer::Stopped(stop) => {
                return Err(refused(stopped(host, engine_state, from, stop)));
            }
            other => return Err(refused(unexpected(&other))),
        };
        Ok(nu::PipelineData::Value(event_id_record(id, call.head), None))
    }
}

/// A stop verdict acted on: the offender stopped, the stop reported, its refusal.
fn stopped(
    host: &HostLink,
    engine_state: &nu::EngineState,
    origin: String,
    stop: HostStop,
) -> String {
    let action = stop_offender(engine_state);
    if stop.notify {
        let report = ChannelSpamErrorEmergency {
            origin,
            hits: stop.hits,
            window_secs: stop.window_secs,
            rate: stop.rate,
            action,
        };
        let _ = host.ask(HostAsk::Stopped(report));
    }
    stop.reason
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

/// The root job: a foreground eval, owning no entry in the jobs table.
const ROOT_JOB_ID: usize = 0;

/// Stop the origin that just crossed the hard threshold.
fn stop_offender(engine_state: &nu::EngineState) -> String {
    engine_state.signals().trigger();
    let job_id = engine_state.current_job.id;
    if job_id.get() == ROOT_JOB_ID {
        return "signals triggered".to_string();
    }
    let mut jobs = engine_state.jobs.lock().unwrap_or_else(|e| e.into_inner());
    match jobs.kill_and_remove(job_id) {
        Ok(()) => format!("signals triggered; job {job_id} killed"),
        Err(e) => format!("signals triggered; job {job_id} removed, kill incomplete: {e}"),
    }
}
