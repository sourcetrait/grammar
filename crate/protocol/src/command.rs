/// The tool's command head, the first word of every command.
pub const INTERFACE: &str = "interface";

/// The words `interface help` answers with a block of its own, no namepath's.
pub const HELP_WORDS: [&str; 2] = [INTERFACE, "grimm"];

/// What one argument holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgShape {
    String,
    Strings,
    Int,
}

impl ArgShape {
    /// The shape's name in a signature line.
    pub fn text(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Strings => "list<string>",
            Self::Int => "int",
        }
    }
}

/// One argument: a positional in its order, or a flag by its name.
#[derive(Debug)]
pub struct ArgSpec {
    pub name: &'static str,
    pub shape: ArgShape,
    pub optional: bool,
    pub summary: &'static str,
}

/// Which command a spec names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    Info,
    Help,
    ProcessList,
    ProcessKill,
    PurviewList,
    PurviewSet,
    PurviewExtend,
    PurviewConfigure,
    ChannelOpen,
    ChannelSync,
    ChannelReady,
    ChannelClose,
    ChannelConfig,
    RemoteList,
    RemoteGet,
    RemoteSet,
    RemoteUnset,
    RemoteListen,
    RemoteConnect,
    RemoteClose,
    EngineList,
    EngineGet,
    EngineSet,
    EngineUnset,
    EngineConnect,
    EngineClose,
    EngineImport,
    EquipNew,
    EquipInstall,
    EquipCheck,
    EquipUninstall,
    EquipScaffold,
    EquipCommit,
}

/// One interface command: its name, arguments, result, and summary.
#[derive(Debug)]
pub struct CommandSpec {
    pub kind: CommandKind,
    pub name: &'static str,
    pub topic: &'static str,
    pub positionals: &'static [ArgSpec],
    pub flags: &'static [ArgSpec],
    pub result: &'static str,
    pub summary: &'static str,
}

impl CommandSpec {
    /// The command's words after `interface`, its name to `--deny`.
    pub fn words(&self) -> &'static str {
        let name: &'static str = self.name;
        &name[INTERFACE.len() + 1..]
    }

    /// The command's signature line: its words, its parameters, its result's kind.
    pub fn signature_text(&self) -> String {
        let positionals = self.positionals.iter().map(|arg| {
            let optional = if arg.optional { "?" } else { "" };
            format!("{}{optional}: {}", arg.name, arg.shape.text())
        });
        let flags = self
            .flags
            .iter()
            .map(|flag| format!("--{}: {}", flag.name, flag.shape.text()));
        let parameters: Vec<String> = positionals.chain(flags).collect();
        format!(
            "{} [{}]: nothing -> {}",
            self.words(),
            parameters.join(", "),
            self.result,
        )
    }
}

const NO_ARGS: &[ArgSpec] = &[];

const HELP_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "namepath",
    shape: ArgShape::String,
    optional: false,
    summary: "A namepath, a pattern ending in `/` or `:`, `*`, `.`, `interface`, or `grimm`.",
}];

const KILL_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "nonce",
    shape: ArgShape::String,
    optional: false,
    summary: "The nonce of the in-flight call to cancel.",
}];

const SET_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "purviews",
    shape: ArgShape::Strings,
    optional: false,
    summary: "The purview ids to put in view, replacing the current set.",
}];

const EXTEND_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "purviews",
    shape: ArgShape::Strings,
    optional: false,
    summary: "Purview ids to add to the current view.",
}];

const CONFIGURE_ARGS: &[ArgSpec] = &[
    ArgSpec {
        name: "purview",
        shape: ArgShape::String,
        optional: false,
        summary: "The purview to set, created if absent; a path-like snake label.",
    },
    ArgSpec {
        name: "namepaths",
        shape: ArgShape::Strings,
        optional: false,
        summary: "Its namepath patterns; an empty list deletes the purview.",
    },
];

const CHANNEL_MODE_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "mode",
    shape: ArgShape::String,
    optional: false,
    summary: "`async` for a Monitor's wss stream, or `sync` for `interface channel sync`.",
}];

const CONFIG_FLAGS: &[ArgSpec] = &[
    ArgSpec {
        name: "spam-warn-window-secs",
        shape: ArgShape::Int,
        optional: true,
        summary: "Window for the soft threshold, in whole seconds.",
    },
    ArgSpec {
        name: "spam-warn-rate",
        shape: ArgShape::Int,
        optional: true,
        summary: "Sends within that window before one warning is issued for an origin.",
    },
    ArgSpec {
        name: "spam-error-window-secs",
        shape: ArgShape::Int,
        optional: true,
        summary: "Window for the hard threshold, in whole seconds.",
    },
    ArgSpec {
        name: "spam-error-rate",
        shape: ArgShape::Int,
        optional: true,
        summary: "Sends within that window before the origin is refused and stopped.",
    },
];

const ALIAS_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "alias",
    shape: ArgShape::String,
    optional: false,
    summary: "The remote's alias, its `[[channel]]` entry's in `remotes.toml`.",
}];

const SET_FLAGS: &[ArgSpec] = &[
    ArgSpec {
        name: "address",
        shape: ArgShape::String,
        optional: true,
        summary: "The `ip:port` a connect dials by default.",
    },
    ArgSpec {
        name: "listen",
        shape: ArgShape::String,
        optional: true,
        summary: "The `ip` or `ip:port` a listen binds by default.",
    },
    ArgSpec {
        name: "cert",
        shape: ArgShape::String,
        optional: true,
        summary: "The peer's certificate, PEM; a new remote needs it.",
    },
];

const LISTEN_FLAGS: &[ArgSpec] = &[ArgSpec {
    name: "on",
    shape: ArgShape::String,
    optional: true,
    summary: "The `ip` or `ip:port` to bind, in place of the entry's.",
}];

const CONNECT_FLAGS: &[ArgSpec] = &[ArgSpec {
    name: "address",
    shape: ArgShape::String,
    optional: true,
    summary: "The `ip:port` to dial, in place of the entry's.",
}];

const CLOSE_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "alias",
    shape: ArgShape::String,
    optional: false,
    summary: "The alias of the open link to close.",
}];

const ENGINE_ALIAS_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "alias",
    shape: ArgShape::String,
    optional: false,
    summary: "The engine's alias, its `[[engine]]` entry's in `remotes.toml`.",
}];

const ENGINE_SET_FLAGS: &[ArgSpec] = &[
    ArgSpec {
        name: "address",
        shape: ArgShape::String,
        optional: true,
        summary: "The host as `<host>[:<port>]`, port 22 by default, an IPv6 host in brackets.",
    },
    ArgSpec {
        name: "user",
        shape: ArgShape::String,
        optional: true,
        summary: "The account the session logs in as.",
    },
    ArgSpec {
        name: "key",
        shape: ArgShape::String,
        optional: true,
        summary: "The private key file the session authenticates with, unencrypted.",
    },
    ArgSpec {
        name: "known-hosts",
        shape: ArgShape::Strings,
        optional: true,
        summary: "The host's public keys as OpenSSH lines, in place of a known_hosts file.",
    },
    ArgSpec {
        name: "known-hosts-file",
        shape: ArgShape::String,
        optional: true,
        summary: "A known_hosts file holding the host's keys, in place of the keys inline.",
    },
    ArgSpec {
        name: "command",
        shape: ArgShape::String,
        optional: true,
        summary: "The worker on the host; `grammar-ssh` on the login's PATH when absent.",
    },
    ArgSpec {
        name: "id",
        shape: ArgShape::String,
        optional: true,
        summary: "The worker's id; the MCP's own when absent.",
    },
    ArgSpec {
        name: "namespace",
        shape: ArgShape::String,
        optional: true,
        summary: "The worker's namespace; the MCP's own when absent.",
    },
    ArgSpec {
        name: "workdir",
        shape: ArgShape::String,
        optional: true,
        summary: "The worker's absolute working directory; the login's own when absent.",
    },
];

const ENGINE_IMPORT_FLAGS: &[ArgSpec] = &[
    ArgSpec {
        name: "from",
        shape: ArgShape::String,
        optional: true,
        summary: "The ssh configuration's host alias to read; the engine's alias when absent.",
    },
    ArgSpec {
        name: "ssh-config",
        shape: ArgShape::String,
        optional: true,
        summary: "The ssh configuration file to read, in place of the user's.",
    },
];

const SOURCE_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "source",
    shape: ArgShape::String,
    optional: false,
    summary: "The equip's source: an `equip/<author>/<name>` directory.",
}];

const SCAFFOLD_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "namepaths",
    shape: ArgShape::Strings,
    optional: false,
    summary: "Namepaths to scaffold into installed equips' rigs: a module or a function.",
}];

const COMMIT_ARGS: &[ArgSpec] = &[ArgSpec {
    name: "namepath",
    shape: ArgShape::String,
    optional: false,
    summary: "The equip to commit: its `<author>/<name>`, or a builtin's name.",
}];

/// Every interface command, in the order the description lists them.
pub const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        kind: CommandKind::Info,
        name: "interface info",
        topic: "General",
        positionals: NO_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "MCP details, Equipment signatures within current purview",
    },
    CommandSpec {
        kind: CommandKind::Help,
        name: "interface help",
        topic: "General",
        positionals: HELP_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "Displays equipment documentation",
    },
    CommandSpec {
        kind: CommandKind::ProcessList,
        name: "interface process list",
        topic: "Process",
        positionals: NO_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "List in-flight MCP tool usage.",
    },
    CommandSpec {
        kind: CommandKind::ProcessKill,
        name: "interface process kill",
        topic: "Process",
        positionals: KILL_ARGS,
        flags: NO_ARGS,
        result: "nothing",
        summary: "Cancel an in-flight usage by its nonce.",
    },
    CommandSpec {
        kind: CommandKind::PurviewList,
        name: "interface purview list",
        topic: "Purview",
        positionals: NO_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "List every configured purview, and what is currently in view.",
    },
    CommandSpec {
        kind: CommandKind::PurviewSet,
        name: "interface purview set",
        topic: "Purview",
        positionals: SET_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "Set the current view to these purviews.",
    },
    CommandSpec {
        kind: CommandKind::PurviewExtend,
        name: "interface purview extend",
        topic: "Purview",
        positionals: EXTEND_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "Bring more purviews into the current view.",
    },
    CommandSpec {
        kind: CommandKind::PurviewConfigure,
        name: "interface purview configure",
        topic: "Purview",
        positionals: CONFIGURE_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "Set a purview's namepath patterns; an empty list deletes it.",
    },
    CommandSpec {
        kind: CommandKind::ChannelOpen,
        name: "interface channel open",
        topic: "Channel",
        positionals: CHANNEL_MODE_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "Opens the Channel",
    },
    CommandSpec {
        kind: CommandKind::ChannelSync,
        name: "interface channel sync",
        topic: "Channel",
        positionals: NO_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "Takes the packets a sync Channel queued since the last sync.",
    },
    CommandSpec {
        kind: CommandKind::ChannelReady,
        name: "interface channel ready",
        topic: "Channel",
        positionals: NO_ARGS,
        flags: NO_ARGS,
        result: "nothing",
        summary: "Verifies Channel activation and begins normal use.",
    },
    CommandSpec {
        kind: CommandKind::ChannelClose,
        name: "interface channel close",
        topic: "Channel",
        positionals: NO_ARGS,
        flags: NO_ARGS,
        result: "nothing",
        summary: "Closes the Channel",
    },
    CommandSpec {
        kind: CommandKind::ChannelConfig,
        name: "interface channel config",
        topic: "Channel",
        positionals: NO_ARGS,
        flags: CONFIG_FLAGS,
        result: "record",
        summary: "Read or adjust the channel's send-rate thresholds at runtime.",
    },
    CommandSpec {
        kind: CommandKind::RemoteList,
        name: "interface remote channel list",
        topic: "Remote",
        positionals: NO_ARGS,
        flags: NO_ARGS,
        result: "table",
        summary: "List the remote channels, configured and live: each one's entry and state.",
    },
    CommandSpec {
        kind: CommandKind::RemoteGet,
        name: "interface remote channel get",
        topic: "Remote",
        positionals: ALIAS_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "One remote channel's entry and state.",
    },
    CommandSpec {
        kind: CommandKind::RemoteSet,
        name: "interface remote channel set",
        topic: "Remote",
        positionals: ALIAS_ARGS,
        flags: SET_FLAGS,
        result: "record",
        summary: "Make or change a remote channel's entry in `remotes.toml`.",
    },
    CommandSpec {
        kind: CommandKind::RemoteUnset,
        name: "interface remote channel unset",
        topic: "Remote",
        positionals: ALIAS_ARGS,
        flags: NO_ARGS,
        result: "nothing",
        summary: "Remove a remote channel's entry from `remotes.toml`; a live link stays.",
    },
    CommandSpec {
        kind: CommandKind::RemoteListen,
        name: "interface remote channel listen",
        topic: "Remote",
        positionals: ALIAS_ARGS,
        flags: LISTEN_FLAGS,
        result: "record",
        summary: "Listen for a remote channel's peer; blocks on the bind, answers its endpoint.",
    },
    CommandSpec {
        kind: CommandKind::RemoteConnect,
        name: "interface remote channel connect",
        topic: "Remote",
        positionals: ALIAS_ARGS,
        flags: CONNECT_FLAGS,
        result: "nothing",
        summary: "Connect to a remote channel's peer; blocks on the connect.",
    },
    CommandSpec {
        kind: CommandKind::RemoteClose,
        name: "interface remote channel close",
        topic: "Remote",
        positionals: CLOSE_ARGS,
        flags: NO_ARGS,
        result: "nothing",
        summary: "Close an open mTLS link to a remote grammar host.",
    },
    CommandSpec {
        kind: CommandKind::EngineList,
        name: "interface remote engine list",
        topic: "Remote",
        positionals: NO_ARGS,
        flags: NO_ARGS,
        result: "table",
        summary: "List the remote engines, configured and live: each one's entry and state.",
    },
    CommandSpec {
        kind: CommandKind::EngineGet,
        name: "interface remote engine get",
        topic: "Remote",
        positionals: ENGINE_ALIAS_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "One remote engine's entry and state.",
    },
    CommandSpec {
        kind: CommandKind::EngineSet,
        name: "interface remote engine set",
        topic: "Remote",
        positionals: ENGINE_ALIAS_ARGS,
        flags: ENGINE_SET_FLAGS,
        result: "record",
        summary: "Make or change a remote engine's entry in `remotes.toml`.",
    },
    CommandSpec {
        kind: CommandKind::EngineUnset,
        name: "interface remote engine unset",
        topic: "Remote",
        positionals: ENGINE_ALIAS_ARGS,
        flags: NO_ARGS,
        result: "nothing",
        summary: "Remove a remote engine's entry from `remotes.toml`; a live session stays.",
    },
    CommandSpec {
        kind: CommandKind::EngineConnect,
        name: "interface remote engine connect",
        topic: "Remote",
        positionals: ENGINE_ALIAS_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "Open a remote engine's session and start its worker now.",
    },
    CommandSpec {
        kind: CommandKind::EngineClose,
        name: "interface remote engine close",
        topic: "Remote",
        positionals: ENGINE_ALIAS_ARGS,
        flags: NO_ARGS,
        result: "nothing",
        summary: "Close a remote engine's session; its worker ends with it.",
    },
    CommandSpec {
        kind: CommandKind::EngineImport,
        name: "interface remote engine import",
        topic: "Remote",
        positionals: ENGINE_ALIAS_ARGS,
        flags: ENGINE_IMPORT_FLAGS,
        result: "record",
        summary: "Make or change a remote engine's entry from an ssh configuration's host.",
    },
    CommandSpec {
        kind: CommandKind::EquipNew,
        name: "interface equip new",
        topic: "Equip",
        positionals: SOURCE_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "Register a new equip at its source and seed its rig.",
    },
    CommandSpec {
        kind: CommandKind::EquipInstall,
        name: "interface equip install",
        topic: "Equip",
        positionals: SOURCE_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "Validate an equip's whole source and install it.",
    },
    CommandSpec {
        kind: CommandKind::EquipCheck,
        name: "interface equip check",
        topic: "Equip",
        positionals: SOURCE_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "Validate an installed equip's source, writing nothing.",
    },
    CommandSpec {
        kind: CommandKind::EquipUninstall,
        name: "interface equip uninstall",
        topic: "Equip",
        positionals: SOURCE_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "Remove an installed equip from the namespace; its source stays.",
    },
    CommandSpec {
        kind: CommandKind::EquipScaffold,
        name: "interface equip scaffold",
        topic: "Equip",
        positionals: SCAFFOLD_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "Scaffold modules / functions (by namepath) into an equip's rig.",
    },
    CommandSpec {
        kind: CommandKind::EquipCommit,
        name: "interface equip commit",
        topic: "Equip",
        positionals: COMMIT_ARGS,
        flags: NO_ARGS,
        result: "record",
        summary: "Commit an equip's lib and rig from its source to the MCP's repository.",
    },
];

/// The spec a command's full name names.
pub fn command_spec(name: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|spec| spec.name == name)
}

/// The spec a deny entry names by the command's words after `interface`.
pub fn command_by_words(words: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|spec| spec.words() == words)
}

/// The commands `denied` leaves, by their words, a `##` heading a topic.
pub fn render_command_list(denied: impl Fn(&str) -> bool) -> String {
    let mut out = String::from("# Command Signatures\n");
    let mut topic = "";
    for spec in COMMANDS.iter().filter(|spec| !denied(spec.words())) {
        if spec.topic != topic {
            topic = spec.topic;
            out.push_str(&format!("## {topic}\n"));
        }
        out.push_str(&format!("- {}\n", spec.name));
    }
    out
}

/// The signatures `denied` leaves, a `###` heading a topic.
pub fn render_signatures(denied: impl Fn(&str) -> bool) -> String {
    let mut out = format!("## Command Signatures\nNote: Each begins with `{INTERFACE} `\n");
    let mut topic = "";
    for spec in COMMANDS.iter().filter(|spec| !denied(spec.words())) {
        if spec.topic != topic {
            topic = spec.topic;
            out.push_str(&format!("### {topic}\n"));
        }
        out.push_str(&format!("{}\n`{}`\n", spec.summary, spec.signature_text()));
    }
    out
}
