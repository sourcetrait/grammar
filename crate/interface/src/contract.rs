//! An interface command held to its contract: one call, its arguments literals.
use crate::*;

/// The name a command is parsed under, which positions its diagnostics.
const COMMAND_FILE: &str = "command";

/// The kind every refusal is reported under.
pub const KIND_INTERFACE: &str = "interface::invalid";

/// A parsed command, by the system that owns it: the routing table.
#[derive(Debug)]
pub enum InterfaceCommand {
    Channel(ChannelCommand),
    Peer(PeerCommand),
    RemoteEngine(RemoteEngineCommand),
    Engine(EngineInterfaceCommand),
}

/// The Channel's five commands, the Channel's system's.
#[derive(Debug)]
pub enum ChannelCommand {
    Open(ChannelOpenParams),
    Sync,
    Ready,
    Close,
    Config(ConfigChannelParams),
}

/// The remote channels' seven commands, the peer links' system's.
#[derive(Debug)]
pub enum PeerCommand {
    List,
    Get(RemoteAliasParams),
    Set(RemoteSetParams),
    Unset(RemoteAliasParams),
    Listen(RemoteListenParams),
    Connect(RemoteConnectParams),
    Close(RemoteChannelCloseParams),
}

/// The remote engines' seven commands, the MCP's own.
#[derive(Debug)]
pub enum RemoteEngineCommand {
    List,
    Get(RemoteAliasParams),
    Set(RemoteEngineSetParams),
    Unset(RemoteAliasParams),
    Connect(RemoteAliasParams),
    Close(RemoteAliasParams),
    Import(RemoteEngineImportParams),
}

/// The engine's commands: info, help, the processes, the purviews, the equips.
#[derive(Debug)]
pub enum EngineInterfaceCommand {
    Info,
    Help(InspectParams),
    ProcessList,
    ProcessKill(KillParams),
    PurviewList,
    PurviewSet(PurviewParams),
    PurviewExtend(PurviewExtendParams),
    PurviewConfigure(PurviewConfigureParams),
    EquipNew(EquipParams),
    EquipInstall(EquipParams),
    EquipCheck(EquipParams),
    EquipUninstall(EquipParams),
    EquipScaffold(NewParams),
    EquipCommit(CommitParams),
}

/// One literal argument's value.
enum Literal {
    Text(String),
    Texts(Vec<String>),
    Whole(i64),
}

/// A call's literal arguments by name, each with its span.
type Literals = HashMap<&'static str, (Literal, nu::Span)>;

/// A refusal before it is positioned: its span, if any, and its message.
type Refusal = (Option<nu::Span>, String);

/// Hold `text` to the contract: one interface call, its arguments literals.
pub fn parse_command(
    engine: &nu::EngineState,
    text: &str,
) -> Result<InterfaceCommand, Vec<Diagnostic>> {
    let mut ws = nu::StateWorkingSet::new(engine);
    let block = nu::parse(&mut ws, Some(COMMAND_FILE), text.as_bytes(), false);
    let start = ws.get_span_for_filename(COMMAND_FILE).map_or(0, |span| span.start);
    let row = |(span, message): Refusal| -> Diagnostic {
        let position = match span {
            Some(span) => {
                let (line, col) = span_to_line_col(text, span.start.saturating_sub(start));
                [wide(line), wide(col)]
            }
            None => [0, 0],
        };
        Diagnostic::error(
            KIND_INTERFACE,
            Some(Source {
                path: None,
                position,
            }),
            message,
        )
    };
    if !ws.parse_errors.is_empty() {
        let mut rows: Vec<Diagnostic> = Vec::new();
        for error in &ws.parse_errors {
            let refused = row((
                Some(error.span()),
                format!("parse error: {}", nuval::NuValError::from(error).short()),
            ));
            if !rows.iter().any(|seen| seen.message == refused.message) {
                rows.push(refused);
            }
        }
        rows.truncate(LINT_VIOLATION_CAP);
        return Err(rows);
    }
    match held_call(&ws, &block, text) {
        Ok(command) => Ok(command),
        Err(refusals) => {
            let mut rows: Vec<Diagnostic> = refusals.into_iter().map(row).collect();
            rows.truncate(LINT_VIOLATION_CAP);
            Err(rows)
        }
    }
}

/// The one call the block holds, mapped to its command.
fn held_call(
    ws: &nu::StateWorkingSet,
    block: &nu::Block,
    text: &str,
) -> Result<InterfaceCommand, Vec<Refusal>> {
    let refuse = |span: Option<nu::Span>, message: String| vec![(span, message)];
    let [pipeline] = block.pipelines.as_slice() else {
        return Err(refuse(
            None,
            format!(
                "one `{INTERFACE}` command, alone; found {} statements",
                block.pipelines.len(),
            ),
        ));
    };
    let element = match pipeline.elements.as_slice() {
        [element] => element,
        [_, next, ..] => {
            return Err(refuse(
                Some(next.expr.span),
                format!("one `{INTERFACE}` command, never piped"),
            ));
        }
        [] => {
            return Err(refuse(None, format!("one `{INTERFACE}` command, alone")));
        }
    };
    if element.redirection.is_some() {
        return Err(refuse(
            Some(element.expr.span),
            format!("an `{INTERFACE}` command takes no redirection"),
        ));
    }
    let nu::Expr::Call(call) = &element.expr.expr else {
        return Err(refuse(
            Some(element.expr.span),
            format!(
                "`{}` is no `{INTERFACE}` command; the commands are in the tool's description",
                text.trim(),
            ),
        ));
    };
    let Some(spec) = command_spec(ws.get_decl(call.decl_id).name()) else {
        return Err(refuse(
            Some(element.expr.span),
            format!("`{}` is no `{INTERFACE}` command", text.trim()),
        ));
    };
    let mut literals: Literals = HashMap::new();
    let mut refusals: Vec<Refusal> = Vec::new();
    let mut index = 0usize;
    for argument in &call.arguments {
        match argument {
            nu::Argument::Positional(expr) => {
                match spec.positionals.get(index) {
                    Some(arg) => match literal(expr, arg) {
                        Ok(value) => {
                            literals.insert(arg.name, (value, expr.span));
                        }
                        Err(refused) => refusals.push(refused),
                    },
                    None => refusals.push((Some(expr.span), "an extra argument".to_string())),
                }
                index += 1;
            }
            nu::Argument::Named((flag, _, value)) => {
                let Some(arg) = spec.flags.iter().find(|arg| arg.name == flag.item) else {
                    refusals.push((
                        Some(flag.span),
                        format!("`{}` takes no flag `--{}`", spec.name, flag.item),
                    ));
                    continue;
                };
                match value {
                    Some(expr) => match literal(expr, arg) {
                        Ok(value) => {
                            literals.insert(arg.name, (value, expr.span));
                        }
                        Err(refused) => refusals.push(refused),
                    },
                    None => refusals.push((
                        Some(flag.span),
                        format!("`--{}` takes a value", arg.name),
                    )),
                }
            }
            nu::Argument::Unknown(expr) | nu::Argument::Spread(expr) => {
                refusals.push((Some(expr.span), "a literal argument, never spread".to_string()));
            }
        }
    }
    if !refusals.is_empty() {
        return Err(refusals);
    }
    command(spec.kind, &mut literals).map_err(|refused| vec![refused])
}

/// An argument's literal, refused when it is anything else.
fn literal(
    expr: &nu::Expression,
    arg: &ArgSpec,
) -> Result<Literal, Refusal> {
    let refused = || {
        (
            Some(expr.span),
            format!(
                "`{}` takes a {} literal, never a variable, an expression, or a spread",
                arg.name,
                match arg.shape {
                    ArgShape::String => "string",
                    ArgShape::Strings => "list-of-strings",
                    ArgShape::Int => "int",
                },
            ),
        )
    };
    match (arg.shape, &expr.expr) {
        (ArgShape::String, nu::Expr::String(text) | nu::Expr::RawString(text)) => {
            Ok(Literal::Text(text.clone()))
        }
        (ArgShape::Int, nu::Expr::Int(whole)) => Ok(Literal::Whole(*whole)),
        (ArgShape::Strings, nu::Expr::List(items)) => {
            let mut texts = Vec::new();
            for item in items {
                let nu::ListItem::Item(item) = item else {
                    return Err(refused());
                };
                match &item.expr {
                    nu::Expr::String(text) | nu::Expr::RawString(text) => texts.push(text.clone()),
                    _ => return Err(refused()),
                }
            }
            Ok(Literal::Texts(texts))
        }
        _ => Err(refused()),
    }
}

/// A string argument; a required one the parse has already held present.
fn text(
    literals: &mut Literals,
    name: &str,
) -> String {
    match literals.remove(name) {
        Some((Literal::Text(text), _)) => text,
        _ => String::new(),
    }
}

/// An optional string flag; absent when not given.
fn optional_text(
    literals: &mut Literals,
    name: &str,
) -> Option<String> {
    match literals.remove(name) {
        Some((Literal::Text(text), _)) => Some(text),
        _ => None,
    }
}

/// A list-of-strings argument; an absent optional one is empty.
fn texts(
    literals: &mut Literals,
    name: &str,
) -> Vec<String> {
    match literals.remove(name) {
        Some((Literal::Texts(texts), _)) => texts,
        _ => Vec::new(),
    }
}

/// An optional list-of-strings flag, absent when not given, so an empty one stays.
fn optional_texts(
    literals: &mut Literals,
    name: &str,
) -> Option<Vec<String>> {
    match literals.remove(name) {
        Some((Literal::Texts(texts), _)) => Some(texts),
        _ => None,
    }
}

/// A whole-number flag in its parameter's range; absent when not given.
fn count<T: TryFrom<i64>>(
    literals: &mut Literals,
    name: &str,
) -> Result<Option<T>, Refusal> {
    match literals.remove(name) {
        None => Ok(None),
        Some((Literal::Whole(whole), span)) => T::try_from(whole).map(Some).map_err(|_| {
            (
                Some(span),
                format!("`--{name}` takes a non-negative whole number in range, not {whole}"),
            )
        }),
        Some((_, span)) => Err((Some(span), format!("`--{name}` takes an int"))),
    }
}

/// The Channel's mode, `sync` or `async`; any other value refused at its position.
fn mode(literals: &mut Literals) -> Result<ChannelMode, Refusal> {
    match literals.remove("mode") {
        Some((Literal::Text(text), span)) => ChannelMode::from_name(&text)
            .ok_or_else(|| (Some(span), format!("`mode` is `sync` or `async`, not `{text}`"))),
        Some((_, span)) => Err((Some(span), "`mode` takes a string".to_string())),
        None => Err((None, "`mode` is required: `sync` or `async`".to_string())),
    }
}

/// The command a spec names, its parameters from the literals.
fn command(
    kind: CommandKind,
    literals: &mut Literals,
) -> Result<InterfaceCommand, Refusal> {
    use EngineInterfaceCommand as E;
    let source = |literals: &mut Literals| EquipParams {
        source: text(literals, "source"),
    };
    let channel = InterfaceCommand::Channel;
    let peer = InterfaceCommand::Peer;
    let remote_engine = InterfaceCommand::RemoteEngine;
    let engine = InterfaceCommand::Engine;
    let alias = |literals: &mut Literals| RemoteAliasParams {
        alias: text(literals, "alias"),
    };
    Ok(match kind {
        CommandKind::Info => engine(E::Info),
        CommandKind::Help => engine(E::Help(InspectParams {
            namepath: text(literals, "namepath"),
        })),
        CommandKind::ProcessList => engine(E::ProcessList),
        CommandKind::ProcessKill => engine(E::ProcessKill(KillParams {
            nonce: text(literals, "nonce"),
        })),
        CommandKind::PurviewList => engine(E::PurviewList),
        CommandKind::PurviewSet => engine(E::PurviewSet(PurviewParams {
            purviews: texts(literals, "purviews"),
        })),
        CommandKind::PurviewExtend => engine(E::PurviewExtend(PurviewExtendParams {
            purviews: texts(literals, "purviews"),
        })),
        CommandKind::PurviewConfigure => engine(E::PurviewConfigure(PurviewConfigureParams {
            purview: text(literals, "purview"),
            namepaths: texts(literals, "namepaths"),
        })),
        CommandKind::ChannelOpen => channel(ChannelCommand::Open(ChannelOpenParams {
            mode: mode(literals)?,
        })),
        CommandKind::ChannelSync => channel(ChannelCommand::Sync),
        CommandKind::ChannelReady => channel(ChannelCommand::Ready),
        CommandKind::ChannelClose => channel(ChannelCommand::Close),
        CommandKind::ChannelConfig => channel(ChannelCommand::Config(ConfigChannelParams {
            spam_warn_window_secs: count(literals, "spam-warn-window-secs")?,
            spam_warn_rate: count(literals, "spam-warn-rate")?,
            spam_error_window_secs: count(literals, "spam-error-window-secs")?,
            spam_error_rate: count(literals, "spam-error-rate")?,
        })),
        CommandKind::RemoteList => peer(PeerCommand::List),
        CommandKind::RemoteGet => peer(PeerCommand::Get(RemoteAliasParams {
            alias: text(literals, "alias"),
        })),
        CommandKind::RemoteSet => peer(PeerCommand::Set(RemoteSetParams {
            alias: text(literals, "alias"),
            address: optional_text(literals, "address"),
            listen: optional_text(literals, "listen"),
            cert: optional_text(literals, "cert"),
        })),
        CommandKind::RemoteUnset => peer(PeerCommand::Unset(RemoteAliasParams {
            alias: text(literals, "alias"),
        })),
        CommandKind::RemoteListen => peer(PeerCommand::Listen(RemoteListenParams {
            alias: text(literals, "alias"),
            on: optional_text(literals, "on"),
        })),
        CommandKind::RemoteConnect => peer(PeerCommand::Connect(RemoteConnectParams {
            alias: text(literals, "alias"),
            address: optional_text(literals, "address"),
        })),
        CommandKind::RemoteClose => peer(PeerCommand::Close(RemoteChannelCloseParams {
            alias: text(literals, "alias"),
        })),
        CommandKind::EngineList => remote_engine(RemoteEngineCommand::List),
        CommandKind::EngineGet => remote_engine(RemoteEngineCommand::Get(alias(literals))),
        CommandKind::EngineSet => remote_engine(RemoteEngineCommand::Set(RemoteEngineSetParams {
            alias: text(literals, "alias"),
            address: optional_text(literals, "address"),
            user: optional_text(literals, "user"),
            key: optional_text(literals, "key"),
            known_hosts: optional_texts(literals, "known-hosts"),
            known_hosts_file: optional_text(literals, "known-hosts-file"),
            command: optional_text(literals, "command"),
            id: optional_text(literals, "id"),
            namespace: optional_text(literals, "namespace"),
            workdir: optional_text(literals, "workdir"),
        })),
        CommandKind::EngineUnset => remote_engine(RemoteEngineCommand::Unset(alias(literals))),
        CommandKind::EngineConnect => remote_engine(RemoteEngineCommand::Connect(alias(literals))),
        CommandKind::EngineClose => remote_engine(RemoteEngineCommand::Close(alias(literals))),
        CommandKind::EngineImport => {
            remote_engine(RemoteEngineCommand::Import(RemoteEngineImportParams {
                alias: text(literals, "alias"),
                from: optional_text(literals, "from"),
                ssh_config: optional_text(literals, "ssh-config"),
            }))
        }
        CommandKind::EquipNew => engine(E::EquipNew(source(literals))),
        CommandKind::EquipInstall => engine(E::EquipInstall(source(literals))),
        CommandKind::EquipCheck => engine(E::EquipCheck(source(literals))),
        CommandKind::EquipUninstall => engine(E::EquipUninstall(source(literals))),
        CommandKind::EquipScaffold => engine(E::EquipScaffold(NewParams {
            namepaths: texts(literals, "namepaths"),
        })),
        CommandKind::EquipCommit => engine(E::EquipCommit(CommitParams {
            namepath: text(literals, "namepath"),
        })),
    })
}
