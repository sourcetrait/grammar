use crate::*;

/// The name write errors give the call's stderr log.
const LOG_NAME: &str = "the call's stderr log";

/// `print` with upstream's signature, writing to the running call's stderr log.
#[derive(Clone)]
pub(crate) struct HostPrint;

impl nu::Command for HostPrint {
    fn name(&self) -> &str {
        "print"
    }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build("print")
            .input_output_types(vec![
                (nu::Type::Nothing, nu::Type::Nothing),
                (nu::Type::Any, nu::Type::Nothing),
            ])
            .allow_variants_without_examples(true)
            .rest("rest", nu::SyntaxShape::Any, "The values to print.")
            .switch(
                "no-newline",
                "Print without inserting a newline for the line ending.",
                Some('n'),
            )
            .switch("stderr", "Print to stderr instead of stdout.", Some('e'))
            .switch(
                "raw",
                "Print without formatting (including binary data).",
                Some('r'),
            )
            .category(nu::Category::Strings)
    }

    fn description(&self) -> &str {
        "Print the given values to this call's stderr log."
    }

    fn extra_description(&self) -> &str {
        "Unlike `echo`, this command does not return any value \
         (`print | describe` will return \"nothing\").\n\
         In the grammar host every `print` goes to the call's stderr log, \
         as nushell's protocol modes send it to stderr."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["display"]
    }

    fn run(
        &self,
        engine_state: &nu::EngineState,
        stack: &mut nu::Stack,
        call: &nu::Call<'_>,
        mut input: nu::PipelineData,
    ) -> Result<nu::PipelineData, nu::ShellError> {
        let args: Vec<nu::Value> = call.rest(engine_state, stack, 0)?;
        let no_newline = call.has_flag(engine_state, stack, "no-newline")?;
        let raw = call.has_flag(engine_state, stack, "raw")?;
        let log = NuapiCall::current(engine_state, stack, call.head)?.stderr_log(call.head)?;
        if !args.is_empty() {
            for arg in args {
                let data = nu::PipelineData::value(arg, None);
                print_to(&log, engine_state, stack, data, no_newline, raw)?;
            }
        } else if !input.is_nothing() {
            if let nu::PipelineData::ByteStream(stream, _) = &mut input
                && let nu::ByteStreamSource::Child(child) = stream.source_mut()
            {
                child.ignore_error(true);
            }
            print_to(&log, engine_state, stack, input, no_newline, raw)?;
        }
        Ok(nu::PipelineData::empty())
    }
}

/// `data` rendered as upstream's `print` renders it, written to `log`.
fn print_to(
    log: &fs::File,
    engine_state: &nu::EngineState,
    stack: &mut nu::Stack,
    data: nu::PipelineData,
    no_newline: bool,
    raw: bool,
) -> Result<(), nu::ShellError> {
    if raw {
        if let nu::PipelineData::Value(nu::Value::Binary { val, .. }, _) = &data {
            let mut writer = log;
            let signals = engine_state.signals();
            return nu::write_all_and_flush(val, &mut writer, LOG_NAME, data.span(), signals);
        }
        return write_values(log, engine_state, data, no_newline);
    }
    match data {
        nu::PipelineData::ByteStream(stream, ..)
            if stream.type_() != nu::ByteStreamType::Binary =>
        {
            stream.write_to(log)
        }
        data => {
            let table = render_table(engine_state, stack, data)?;
            write_values(log, engine_state, table, no_newline)
        }
    }
}

/// `data` through nushell's `table` command, as upstream's `print` renders it.
fn render_table(
    engine_state: &nu::EngineState,
    stack: &mut nu::Stack,
    data: nu::PipelineData,
) -> Result<nu::PipelineData, nu::ShellError> {
    let Some(decl_id) = engine_state.table_decl_id else {
        return Ok(data);
    };
    let command = engine_state.get_decl(decl_id);
    if command.block_id().is_some() {
        return Ok(data);
    }
    let call = nu::AstCall::new(nu::Span::new(0, 0));
    command.run(engine_state, stack, &(&call).into(), data)
}

/// Each value of `data` as text to `log`, newline-ended unless `no_newline`.
fn write_values(
    log: &fs::File,
    engine_state: &nu::EngineState,
    data: nu::PipelineData,
    no_newline: bool,
) -> Result<(), nu::ShellError> {
    let span = data.span();
    if let nu::PipelineData::ByteStream(stream, ..) = data {
        return stream.write_to(log);
    }
    let config = engine_state.get_config();
    let mut writer = log;
    for item in data {
        let mut out = if let nu::Value::Error { error, .. } = item {
            return Err(*error);
        } else {
            item.to_expanded_string("\n", config)
        };
        if !no_newline {
            out.push('\n');
        }
        nu::write_all_and_flush(out, &mut writer, LOG_NAME, span, engine_state.signals())?;
    }
    Ok(())
}
