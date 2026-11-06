//! The interface's parse held to its contract, each command to its owning system.
use crate::*;
use sourcetrait_grammar_protocol::render_signatures;

/// An engine with every command.
fn engine() -> nu::EngineState {
    interface_engine(|_| false)
}

/// A command's parse against the full engine.
fn parse(text: &str) -> Result<InterfaceCommand, Vec<Diagnostic>> {
    parse_command(&engine(), text)
}

/// The kinds and messages of a refusal, panicking on a parse that held.
fn refused(text: &str) -> Vec<(String, String)> {
    match parse(text) {
        Ok(command) => panic!("`{text}` held as {command:?}"),
        Err(rows) => rows.into_iter().map(|row| (row.kind, row.message)).collect(),
    }
}

/// The first refusal's message, every row asserted `interface::invalid`.
fn refusal(text: &str) -> String {
    let rows = refused(text);
    assert!(
        rows.iter().all(|(kind, _)| kind == KIND_INTERFACE),
        "`{text}`: every row is {KIND_INTERFACE}; got {rows:?}",
    );
    rows.first().map(|(_, message)| message.clone()).unwrap_or_default()
}

/// A spec's smallest command: each required positional a sample.
fn sample(spec: &CommandSpec) -> String {
    let mut text = spec.name.to_string();
    for arg in spec.positionals.iter().filter(|arg| !arg.optional) {
        text.push_str(match arg.shape {
            ArgShape::String => " 'x'",
            ArgShape::Strings => " ['x']",
            ArgShape::Int => " 1",
        });
    }
    text
}

/// The kind a parsed command is, by the system that owns it.
fn kind_of(command: &InterfaceCommand) -> CommandKind {
    use EngineInterfaceCommand as E;
    match command {
        InterfaceCommand::Engine(command) => match command {
            E::Info => CommandKind::Info,
            E::Help(_) => CommandKind::Help,
            E::ProcessList => CommandKind::ProcessList,
            E::ProcessKill(_) => CommandKind::ProcessKill,
            E::PurviewList => CommandKind::PurviewList,
            E::PurviewSet(_) => CommandKind::PurviewSet,
            E::PurviewExtend(_) => CommandKind::PurviewExtend,
            E::PurviewConfigure(_) => CommandKind::PurviewConfigure,
            E::EquipNew(_) => CommandKind::EquipNew,
            E::EquipInstall(_) => CommandKind::EquipInstall,
            E::EquipCheck(_) => CommandKind::EquipCheck,
            E::EquipUninstall(_) => CommandKind::EquipUninstall,
            E::EquipScaffold(_) => CommandKind::EquipScaffold,
            E::EquipCommit(_) => CommandKind::EquipCommit,
        },
        InterfaceCommand::Channel(command) => match command {
            ChannelCommand::Open => CommandKind::ChannelOpen,
            ChannelCommand::Sync => CommandKind::ChannelSync,
            ChannelCommand::Ready => CommandKind::ChannelReady,
            ChannelCommand::Close => CommandKind::ChannelClose,
            ChannelCommand::Config(_) => CommandKind::ChannelConfig,
        },
        InterfaceCommand::Peer(command) => match command {
            PeerCommand::List => CommandKind::RemoteList,
            PeerCommand::Get(_) => CommandKind::RemoteGet,
            PeerCommand::Set(_) => CommandKind::RemoteSet,
            PeerCommand::Unset(_) => CommandKind::RemoteUnset,
            PeerCommand::Listen(_) => CommandKind::RemoteListen,
            PeerCommand::Connect(_) => CommandKind::RemoteConnect,
            PeerCommand::Close(_) => CommandKind::RemoteClose,
        },
        InterfaceCommand::Chat(command) => match command {
            ChatCommand::Send(_) => CommandKind::ChatSend,
            ChatCommand::Read(_) => CommandKind::ChatRead,
        },
        InterfaceCommand::RemoteEngine(command) => match command {
            RemoteEngineCommand::List => CommandKind::EngineList,
            RemoteEngineCommand::Get(_) => CommandKind::EngineGet,
            RemoteEngineCommand::Set(_) => CommandKind::EngineSet,
            RemoteEngineCommand::Unset(_) => CommandKind::EngineUnset,
            RemoteEngineCommand::Connect(_) => CommandKind::EngineConnect,
            RemoteEngineCommand::Close(_) => CommandKind::EngineClose,
            RemoteEngineCommand::Import(_) => CommandKind::EngineImport,
        },
    }
}

/// A chat send takes its three texts, a read its id and page, 0 when absent.
#[test]
fn a_chat_parses_its_texts_and_a_read_its_page() {
    match parse("interface chat send 'abc' 'a summary' 'the message'") {
        Ok(InterfaceCommand::Chat(ChatCommand::Send(p))) => assert_eq!(
            (p.mcp_nom.as_str(), p.summary.as_str(), p.message.as_str()),
            ("abc", "a summary", "the message"),
        ),
        other => panic!("a chat send, not {other:?}"),
    }
    match parse("chat read 'xyz'") {
        Ok(InterfaceCommand::Chat(ChatCommand::Read(p))) => assert_eq!((p.id.as_str(), p.page), ("xyz", 0)),
        other => panic!("a chat read, not {other:?}"),
    }
    match parse("chat read 'xyz' 2") {
        Ok(InterfaceCommand::Chat(ChatCommand::Read(p))) => assert_eq!(p.page, 2),
        other => panic!("a read of page 2, not {other:?}"),
    }
    assert!(!refusal("chat read 'xyz' 'two'").is_empty(), "a page is an int");
    assert!(!refusal("chat send 'abc' 'a summary'").is_empty(), "a chat needs its message");
}

/// Each command's smallest form parses to its own kind.
#[test]
fn every_command_parses_to_its_kind() {
    for spec in COMMANDS {
        let text = sample(spec);
        let command = parse(&text).unwrap_or_else(|rows| panic!("`{text}` refused: {rows:?}"));
        assert_eq!(kind_of(&command), spec.kind, "`{text}`");
    }
}

/// No command's name or argument holds an underscore.
#[test]
fn no_command_name_holds_an_underscore() {
    for spec in COMMANDS {
        assert!(!spec.name.contains('_'), "{}", spec.name);
        for arg in spec.positionals.iter().chain(spec.flags) {
            assert!(!arg.name.contains('_'), "{}: {}", spec.name, arg.name);
        }
    }
}

/// A command's arguments reach its parameters as written.
#[test]
fn arguments_reach_the_tools_parameters() {
    use EngineInterfaceCommand as E;
    match parse("interface purview configure 'iter/one' ['acme/geo:', '@core']") {
        Ok(InterfaceCommand::Engine(E::PurviewConfigure(p))) => {
            assert_eq!(p.purview, "iter/one");
            assert_eq!(p.namepaths, vec!["acme/geo:".to_string(), "@core".to_string()]);
        }
        other => panic!("got {other:?}"),
    }
    match parse("interface info") {
        Ok(InterfaceCommand::Engine(E::Info)) => {}
        other => panic!("got {other:?}"),
    }
    assert!(!refused("interface info ['default']").is_empty(), "info takes no argument");
    match parse("interface channel config --spam-warn-rate 12 --spam-error-window-secs 30") {
        Ok(InterfaceCommand::Channel(ChannelCommand::Config(p))) => {
            assert_eq!(p.spam_warn_rate, Some(12));
            assert_eq!(p.spam_error_window_secs, Some(30));
            assert_eq!(p.spam_warn_window_secs, None);
            assert_eq!(p.spam_error_rate, None);
        }
        other => panic!("got {other:?}"),
    }
    match parse("interface equip install r#'/tmp/a b/equip/acme/geo'#") {
        Ok(InterfaceCommand::Engine(E::EquipInstall(p))) => {
            assert_eq!(p.source, "/tmp/a b/equip/acme/geo");
        }
        other => panic!("got {other:?}"),
    }
    match parse("interface help grammar:eye:git") {
        Ok(InterfaceCommand::Engine(E::Help(p))) => assert_eq!(p.namepath, "grammar:eye:git"),
        other => panic!("got {other:?}"),
    }
}

/// A remote channel command takes its flags, and only its own.
#[test]
fn a_remote_takes_its_flags() {
    match parse("interface remote channel listen 'peer' --on '127.0.0.1'") {
        Ok(InterfaceCommand::Peer(PeerCommand::Listen(p))) => {
            assert_eq!(p.alias, "peer");
            assert_eq!(p.on.as_deref(), Some("127.0.0.1"));
        }
        other => panic!("got {other:?}"),
    }
    match parse("interface remote channel listen 'peer'") {
        Ok(InterfaceCommand::Peer(PeerCommand::Listen(p))) => {
            assert!(p.on.is_none(), "a flag absent is unset");
        }
        other => panic!("got {other:?}"),
    }
    match parse("interface remote channel set 'peer' --cert '~/k/p.pem' --address '127.0.0.1:9'") {
        Ok(InterfaceCommand::Peer(PeerCommand::Set(p))) => {
            assert_eq!(p.cert.as_deref(), Some("~/k/p.pem"));
            assert_eq!(p.address.as_deref(), Some("127.0.0.1:9"));
            assert_eq!(p.listen, None);
        }
        other => panic!("got {other:?}"),
    }
    match parse("interface remote channel connect 'peer' --address '127.0.0.1:9'") {
        Ok(InterfaceCommand::Peer(PeerCommand::Connect(p))) => {
            assert_eq!(p.address.as_deref(), Some("127.0.0.1:9"));
        }
        other => panic!("got {other:?}"),
    }
    assert!(!refused("interface remote channel listen 'peer' --force").is_empty(), "no `--force`");
    assert!(!refused("interface remote channel open 'peer'").is_empty(), "`open` is retired");
    assert!(
        !refused("interface remote list").is_empty(),
        "the commands sit under `remote channel`"
    );
}

/// A remote engine command takes its flags, known hosts as a list among them.
#[test]
fn a_remote_engine_takes_its_flags() {
    let set = "interface remote engine set 'lab' --platform 'linux' --address 'lab.example:2222' --jump 'gate' \
               --known-hosts ['ssh-ed25519 AAAA one', 'ssh-rsa AAAA other'] --workdir '/srv/work'";
    match parse(set) {
        Ok(InterfaceCommand::RemoteEngine(RemoteEngineCommand::Set(p))) => {
            assert_eq!(p.alias, "lab");
            assert_eq!(p.platform.as_deref(), Some("linux"));
            assert_eq!(p.address.as_deref(), Some("lab.example:2222"));
            assert_eq!(p.jump.as_deref(), Some("gate"));
            assert_eq!(
                p.known_hosts,
                Some(vec!["ssh-ed25519 AAAA one".to_string(), "ssh-rsa AAAA other".to_string()])
            );
            assert_eq!(p.workdir.as_deref(), Some("/srv/work"));
            assert_eq!((p.user, p.key, p.command, p.id, p.namespace), (None, None, None, None, None));
            assert_eq!(p.known_hosts_file, None);
        }
        other => panic!("got {other:?}"),
    }
    match parse("interface remote engine set 'lab' --known-hosts []") {
        Ok(InterfaceCommand::RemoteEngine(RemoteEngineCommand::Set(p))) => {
            assert_eq!(p.known_hosts, Some(Vec::new()), "an empty list given is kept, for the check");
            assert_eq!(p.platform, None, "a flag not given is no change");
        }
        other => panic!("got {other:?}"),
    }
    match parse("interface remote engine set 'ai/lab' --known-hosts-file 'remote/ai/lab/known_hosts'") {
        Ok(InterfaceCommand::RemoteEngine(RemoteEngineCommand::Set(p))) => {
            assert_eq!(p.alias, "ai/lab");
            assert_eq!(p.known_hosts_file.as_deref(), Some("remote/ai/lab/known_hosts"));
            assert_eq!(p.known_hosts, None);
        }
        other => panic!("got {other:?}"),
    }
    match parse("interface remote engine import 'lab' --from 'labssh' --ssh-config '/tmp/ssh/config'") {
        Ok(InterfaceCommand::RemoteEngine(RemoteEngineCommand::Import(p))) => {
            assert_eq!(p.alias, "lab");
            assert_eq!(p.from.as_deref(), Some("labssh"));
            assert_eq!(p.ssh_config.as_deref(), Some("/tmp/ssh/config"));
        }
        other => panic!("got {other:?}"),
    }
    assert!(!refused("interface remote engine set 'lab' --port 22").is_empty(), "the retired port flag");
    assert!(!refused("interface remote engine set 'lab' --host-keys []").is_empty(), "the retired keys flag");
    assert!(!refused("interface remote engine set 'lab' --cert 'x'").is_empty(), "a channel's flag");
    assert!(!refused("interface remote engine listen 'lab'").is_empty(), "no `listen` for an engine");
}

/// A text naming no command is refused as no `interface` command.
#[test]
fn an_unknown_command_is_refused() {
    for text in ["interface", "interface purview", "interface nosuch thing", "ls", "nu -c 'ls'"] {
        let message = refusal(text);
        assert!(message.contains("no `interface` command"), "`{text}`: {message}");
    }
}

/// One command, alone, never piped.
#[test]
fn one_command_alone_never_piped() {
    assert!(refusal("interface info; interface purview list").contains("alone"));
    assert!(refusal("interface info\ninterface purview list").contains("alone"));
    assert!(refusal("interface info | interface purview list").contains("never piped"));
    assert!(refusal("").contains("alone"));
}

/// Only literals reach a command: no variable, expression, or spread.
#[test]
fn only_literals_reach_a_command() {
    for text in [
        "interface help $nu",
        "interface help $env.HOME",
        "interface help $in",
        "interface help $\"x(1)\"",
        "interface purview set [$nu.home-dir]",
        "interface purview set [...['a']]",
    ] {
        assert!(!refused(text).is_empty(), "`{text}` must be refused");
    }
    let rows = refused("interface help (interface info)");
    assert!(!rows.is_empty());
}

/// A redirection is refused.
#[test]
fn a_redirection_is_refused() {
    assert!(!refused("interface info o> out.txt").is_empty());
    assert!(!refused("interface info e> err.txt").is_empty());
}

/// A missing or mistyped argument is refused at its position.
#[test]
fn a_missing_or_mistyped_argument_is_refused_at_its_position() {
    let rows = parse("interface help").expect_err("help takes a namepath");
    assert_eq!(rows[0].kind, KIND_INTERFACE);
    assert!(
        rows[0].message.contains("Missing required positional argument: missing namepath"),
        "{rows:?}"
    );
    assert!(!rows[0].message.contains("Span["), "no engine offsets: {rows:?}");
    let rows = parse("interface purview set 'x'").expect_err("purviews is a list");
    assert_eq!(rows[0].kind, KIND_INTERFACE);
    let position = rows[0].source.as_ref().expect("a source").position;
    assert_eq!(position[0], 1, "{rows:?}");
    assert!(position[1] > 1, "{rows:?}");
    let rows = parse("interface channel config --spam-warn-rate abc").expect_err("a rate is an int");
    assert_eq!(rows[0].kind, KIND_INTERFACE);
    match parse("interface process kill ['a']") {
        Ok(InterfaceCommand::Engine(EngineInterfaceCommand::ProcessKill(p))) => {
            assert_eq!(p.nonce, "['a']", "a string takes the token");
        }
        other => panic!("got {other:?}"),
    }
}

/// The Channel opens with no mode, the host choosing one for the caller's harness.
#[test]
fn the_channel_opens_with_no_mode() {
    match parse("interface channel open") {
        Ok(InterfaceCommand::Channel(ChannelCommand::Open)) => {}
        other => panic!("got {other:?}"),
    }
    for text in ["interface channel open sync", "interface channel open 'async'"] {
        let rows = refused(text);
        assert!(
            rows.iter().all(|(kind, _)| kind == KIND_INTERFACE),
            "`{text}`: a mode is no argument; got {rows:?}",
        );
    }
}

/// A flag is checked for its range, and an unknown one refused.
#[test]
fn a_flag_is_checked_for_its_range() {
    let message = refusal("interface channel config --spam-warn-rate -1");
    assert!(message.contains("non-negative"), "{message}");
    assert!(!refused("interface channel config --nosuch 1").is_empty());
    assert!(!refused("interface info --help").is_empty());
}

/// A subcommand parses with or without its `interface` word, the word taken whole.
#[test]
fn a_subcommand_parses_with_or_without_its_head() {
    for spec in COMMANDS {
        let full = sample(spec);
        let bare = full.strip_prefix(&format!("{INTERFACE} ")).expect("a full name").to_string();
        for text in [full.as_str(), bare.as_str()] {
            let parsed = parse(text).unwrap_or_else(|rows| panic!("`{text}`: {rows:?}"));
            assert_eq!(kind_of(&parsed), spec.kind, "{text}");
        }
    }
    let message = refusal("interfaceinfo");
    assert!(message.contains("no `interface` command"), "the head is a whole word; {message}");
}

/// A refusal is positioned in the subcommand as it was sent, either form.
#[test]
fn a_refusal_is_positioned_in_the_text_as_sent() {
    for text in [
        "channel config --spam-warn-rate -1",
        "interface channel config --spam-warn-rate -1",
        "  channel config --spam-warn-rate -1",
    ] {
        let rows = parse(text).expect_err("a negative rate");
        let col = text.find("-1").expect("the value") as u64 + 1;
        assert_eq!(rows[0].source.as_ref().map(|s| s.position), Some([1, col]), "{text}");
    }
}

/// A denied command is unknown to the parse and absent from the signatures.
#[test]
fn a_denied_command_is_unknown_and_undocumented() {
    let denied = |words: &str| words == "purview configure";
    let engine = interface_engine(denied);
    let rows = parse_command(&engine, "interface purview configure x ['y']")
        .expect_err("a denied command is unknown");
    assert!(rows[0].message.contains("no `interface` command"), "{rows:?}");
    assert!(parse_command(&engine, "interface purview list").is_ok());
    let rendered = render_signatures(denied);
    assert!(!rendered.contains("`purview configure ["), "{rendered}");
    assert!(rendered.contains("`purview list ["), "{rendered}");
}

/// The signatures cover every command, by topic.
#[test]
fn the_signatures_cover_every_command_by_topic() {
    let rendered = render_signatures(|_| false);
    assert!(
        rendered.starts_with(
            "## Command Signatures\n### General\n"
        ),
        "{rendered}",
    );
    for spec in COMMANDS {
        let entry = format!("{}\n`{}`\n", spec.summary, spec.signature_text());
        assert!(rendered.contains(&entry), "{}", spec.name);
    }
    assert!(rendered.contains(
        "Set a purview's namepath patterns; an empty list deletes it.\n\
         `purview configure [purview: string, namepaths: list<string>]: nothing -> record`\n"
    ));
    assert!(rendered.contains("--spam-warn-rate: int"), "{rendered}");
    assert!(
        rendered.contains("`remote channel listen [alias: string, --on: string]: nothing -> record`"),
        "a remote channel command renders its words; {rendered}",
    );
    assert!(!rendered.contains("def ") && !rendered.contains("- `"), "{rendered}");
}

/// Every summary fits a line, and every result is a kind.
#[test]
fn every_summary_fits_a_line_and_every_result_is_a_kind() {
    for spec in COMMANDS {
        assert!(spec.summary.chars().count() <= 80, "{}: {}", spec.name, spec.summary);
        assert!(
            ["record", "table", "nothing"].contains(&spec.result),
            "{}: {}",
            spec.name,
            spec.result,
        );
    }
}
