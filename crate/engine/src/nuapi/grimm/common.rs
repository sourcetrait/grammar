use crate::*;

/// The file the embedded API appends to, inside this call's nonce log dir.
pub(crate) const DEBUG_FILE: &str = "debug.nuonl";

/// The call's stderr log, inside its nonce log dir.
pub(crate) const STDERR_FILE: &str = "stderr";

/// The call's stdout log, inside its nonce log dir.
pub(crate) const STDOUT_FILE: &str = "stdout";

/// The call a host decl acts for: where its artifacts go, and its origin.
pub(crate) struct NuapiCall {
    log_dir: PathBuf,
    origin: String,
}

impl NuapiCall {
    /// The call whose nonce the stack's `$env.NONCE` names, an error outside one.
    pub(crate) fn current(
        engine_state: &nu::EngineState,
        stack: &nu::Stack,
        span: nu::Span,
    ) -> Result<Self, nu::ShellError> {
        let nonce = stack
            .get_env_var(engine_state, "NONCE")
            .and_then(|value| value.as_str().ok())
            .ok_or_else(|| {
                nu::GenericError::new("no call here", "`$env.NONCE` names no call", span)
            })?;
        let log_dir = call_log_dir(nonce).ok_or_else(|| {
            nu::GenericError::new(
                "no call log here",
                format!("no call's log dir answers to the nonce `{nonce}`"),
                span,
            )
        })?;
        Ok(Self {
            log_dir,
            origin: format!("thread/{nonce}"),
        })
    }

    /// This call's stderr log, opened to append after every other writer.
    pub(crate) fn stderr_log(&self, span: nu::Span) -> Result<fs::File, nu::ShellError> {
        let path = self.log_dir.join(STDERR_FILE);
        fs::OpenOptions::new().append(true).open(&path).map_err(|e| {
            nu::GenericError::new(
                format!("cannot open {}", path.display()),
                e.to_string(),
                span,
            )
            .into()
        })
    }

    /// The host-stamped `from` for anything this eval emits.
    pub(crate) fn origin(&self) -> String {
        self.origin.clone()
    }

    /// Append `data` to `<log_dir>/debug.nuonl` as ONE line of NUON.
    pub(crate) fn append_debug(
        &self,
        data: &nu::Value,
        span: nu::Span,
    ) -> Result<(), nu::ShellError> {
        let rendered = nu::to_nuon(&nu::EngineState::new(), data, nu::ToNuonConfig::default())
            .map_err(|e| {
                nu::GenericError::new("cannot render the value as NUON", e.to_string(), span)
            })?;
        let line = rendered.replace('\n', "\\n").replace('\r', "\\r");
        append_line(&self.log_dir.join(DEBUG_FILE), &line).map_err(|e| {
            nu::GenericError::new(format!("cannot append to {DEBUG_FILE}"), e.to_string(), span)
        })?;
        Ok(())
    }
}

/// Append one already-rendered NUON line, creating the parent dir on demand.
fn append_line(
    path: &Path,
    line: &str,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(line.as_bytes())?;
    f.write_all(b"\n")?;
    Ok(())
}

/// Enforce the declared `oneof<record, table>` at RUNTIME.
pub(crate) fn require_record_or_table(
    data: &nu::Value,
    span: nu::Span,
) -> Result<(), nu::ShellError> {
    match data {
        nu::Value::Record { .. } => Ok(()),
        nu::Value::List { vals, .. }
            if vals.iter().all(|v| matches!(v, nu::Value::Record { .. })) =>
        {
            Ok(())
        }
        other => Err(nu::GenericError::new(
            "expected a record or a table",
            format!("got {}", other.get_type()),
            span,
        )
        .into()),
    }
}

/// The `data` positional shared by the `grimm *` decls: record or table.
pub(crate) fn data_shape() -> nu::SyntaxShape {
    nu::SyntaxShape::OneOf(vec![
        nu::SyntaxShape::Record(nu::CollectionColumns::from(vec![])),
        nu::SyntaxShape::Table(nu::CollectionColumns::from(vec![])),
    ])
}

/// The nullable `data` positional shared by the `grimm *` decls: record or table.
#[allow(unused)]
pub(crate) fn optional_data_shape() -> nu::SyntaxShape {
    nu::SyntaxShape::OneOf(vec![
        nu::SyntaxShape::Record(nu::CollectionColumns::from(vec![])),
        nu::SyntaxShape::Table(nu::CollectionColumns::from(vec![])),
        nu::SyntaxShape::Nothing,
    ])
}

/// The record every `grimm` send returns: the message id under `event_id`.
pub(crate) fn event_id_record(
    id: impl Into<String>,
    span: nu::Span,
) -> nu::Value {
    nu::Value::record(nu::record! { "event_id" => nu::Value::string(id, span) }, span)
}

/// The output type every `grimm` send declares: `record<event_id: string>`.
pub(crate) fn event_id_type() -> nu::Type {
    nu::Type::Record(nu::CollectionColumns::from(vec![(
        "event_id".to_string(),
        nu::Type::String,
    )]))
}

/// The word `interface help` takes for the `grimm` commands' signatures.
pub const GRIMM: &str = "grimm";

/// The `grimm *` decls, in the order the signature block lists them.
fn grimm_decls(host: Option<&HostLink>) -> Vec<Box<dyn nu::Command>> {
    let link = || host.cloned();
    vec![
        Box::new(GrimmDbg),
        Box::new(GrimmChannelSend { host: link() }),
        Box::new(GrimmGetConfigAll { host: link() }),
        Box::new(GrimmGetConfig { host: link() }),
        Box::new(GrimmPinConfig { host: link() }),
        Box::new(GrimmRemoteChannelSend { host: link() }),
        Box::new(GrimmRemoteChannelSendWith { host: link() }),
    ]
}

/// Register the `grimm *` decls into the working set an eval will parse with.
pub(crate) fn register_nuapi(
    working_set: &mut nu::StateWorkingSet,
    host: Option<&HostLink>,
) {
    for decl in grimm_decls(host) {
        working_set.add_decl(decl);
    }
}

/// The host a decl asks, or the refusal of a decl that has none.
pub(crate) fn asked_host<'a>(
    host: &'a Option<HostLink>,
    command: &str,
    span: nu::Span,
) -> Result<&'a HostLink, nu::ShellError> {
    host.as_ref().ok_or_else(|| {
        nu::GenericError::new(
            format!("{command} failed"),
            "the engine has no host here".to_string(),
            span,
        )
        .into()
    })
}

/// The refusal of an answer its ask never takes.
pub(crate) fn unexpected(answer: &HostAnswer) -> String {
    format!("the engine's host answered out of kind: {answer:?}")
}

/// A value as an ask's data; one nuval can't carry fails as its render.
pub(crate) fn sendable(value: nu::Value) -> Result<NuVal, String> {
    NuVal::try_from_nu(value.clone()).map_err(|e| match render_nuon(&value) {
        Err(render) => render,
        Ok(_) => e.to_string(),
    })
}

/// The `grimm` commands' signatures: a summary, then a signature line, each.
pub fn grimm_signatures() -> String {
    let mut out = String::from("## grimm Signatures\n");
    for decl in grimm_decls(None) {
        out.push_str(&format!(
            "{}\n`{}`\n",
            decl.description(),
            signature_text(&decl.signature()),
        ));
    }
    out
}

/// A signature's line: its words, its parameters, its input and output types.
fn signature_text(signature: &nu::Signature) -> String {
    let required = signature
        .required_positional
        .iter()
        .map(|arg| format!("{}: {}", arg.name, arg.shape));
    let optional = signature
        .optional_positional
        .iter()
        .map(|arg| format!("{}?: {}", arg.name, arg.shape));
    let rest = signature
        .rest_positional
        .iter()
        .map(|arg| format!("...{}: {}", arg.name, arg.shape));
    let flags = signature
        .named
        .iter()
        .filter(|flag| flag.long != "help")
        .map(|flag| match &flag.arg {
            Some(shape) => format!("--{}: {}", flag.long, shape),
            None => format!("--{}", flag.long),
        });
    let parameters: Vec<String> = required.chain(optional).chain(rest).chain(flags).collect();
    let types: Vec<String> = signature
        .input_output_types
        .iter()
        .map(|(input, output)| format!("{input} -> {output}"))
        .collect();
    format!(
        "{} [{}]: {}",
        signature.name,
        parameters.join(", "),
        types.join(", "),
    )
}
