//! A remote engine's entry: its host, how the host is trusted, and what runs there.
use crate::*;

/// The SSH port an address takes when it names none.
pub const SSH_PORT: u16 = 22;

/// grammar-ssh's command when an entry names none, found on the login's `PATH`.
pub const GRAMMAR_SSH_COMMAND: &str = "grammar-ssh";

/// The most characters an engine's alias takes, so a packet's `from` stays short.
pub const ALIAS_MAX_CHARS: usize = 128;

/// The host's operating system, as its entry names it.
#[cereal::derived(Data, Copy, Eq)]
pub enum Platform {
    Windows,
    Linux,
    Macos,
}

impl Platform {
    /// Every platform's name, as an entry writes it.
    pub const NAMES: [&'static str; 3] = ["windows", "linux", "macos"];

    /// The platform a name gives: `windows`, `linux`, or `macos`.
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            "macos" => Ok(Self::Macos),
            _ => Err(format!("`{name}` is no platform: `windows`, `linux`, or `macos`")),
        }
    }

    /// The platform's name, as an entry writes it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::Macos => "macos",
        }
    }

    /// Whether the host's login takes the Unix wrapper and Unix paths.
    pub fn is_unix(self) -> bool {
        !matches!(self, Self::Windows)
    }
}

/// The host a session reaches its engine's host through, from the jump's entry.
#[cereal::derived(Data, Eq)]
pub struct Jump {
    /// The jump's own alias, which a refusal on its hop names.
    pub alias: String,
    /// The jump host's name or address.
    pub host: String,
    pub port: u16,
    /// The account the jump's session logs in as.
    pub user: String,
    /// The private key file the jump's session authenticates with, on the MCP's host.
    pub key: String,
    /// The jump host's public keys as OpenSSH lines, one of which it must present.
    pub host_keys: Vec<String>,
}

impl Jump {
    /// The jump an engine's resolved endpoint makes: its host, its login, its keys.
    pub fn of(endpoint: &Endpoint) -> Self {
        Self {
            alias: endpoint.alias.clone(),
            host: endpoint.host.clone(),
            port: endpoint.port,
            user: endpoint.user.clone(),
            key: endpoint.key.clone(),
            host_keys: endpoint.host_keys.clone(),
        }
    }

    /// The jump held to its rules, each refusal naming the jump and the field.
    pub fn check(&self) -> Result<(), String> {
        check_alias(&self.alias)?;
        check_login(&self.alias, &self.host, self.port, &self.user, &self.key, &self.host_keys)
    }

    /// Where the jump is, as a refusal on its hop names it.
    pub fn place(&self) -> String {
        format!("the jump `{}` at {}:{}", self.alias, self.host, self.port)
    }

    /// The jump host's pinned keys, each parsed.
    pub(crate) fn pinned_keys(&self) -> Result<Vec<keys::PublicKey>, String> {
        self.host_keys.iter().map(|line| pinned_key(line)).collect()
    }
}

/// A remote engine as the MCP's entry resolves it, each field checked by `check`.
#[cereal::derived(Data, Eq)]
pub struct Endpoint {
    /// The name a tool's `host` gives the engine.
    pub alias: String,
    /// The host's operating system, which decides the exec command and the paths.
    pub platform: Platform,
    /// The host's name or address.
    pub host: String,
    pub port: u16,
    /// The account the session logs in as.
    pub user: String,
    /// The private key file the session authenticates with, on the MCP's host.
    pub key: String,
    /// The host's public keys as OpenSSH lines, one of which it must present.
    pub host_keys: Vec<String>,
    /// The host the session reaches this one through, where the entry names a jump.
    pub jump: Option<Jump>,
    /// grammar-ssh's command: a name the login's `PATH` finds, or an absolute path.
    pub command: String,
    /// grammar-ssh's id.
    pub id: String,
    /// grammar-ssh's namespace.
    pub namespace: String,
    /// grammar-ssh's working directory on the host; the session's own when none.
    pub workdir: Option<String>,
}

impl Endpoint {
    /// The entry held to its rules, each refusal naming the field it is about.
    pub fn check(&self) -> Result<(), String> {
        check_alias(&self.alias)?;
        check_login(&self.alias, &self.host, self.port, &self.user, &self.key, &self.host_keys)?;
        let fields = [
            ("command", check_command(&self.command, self.platform)),
            ("workdir", self.workdir.as_deref().map_or(Ok(()), |path| check_path(path, self.platform))),
            ("id or namespace", check_instance(&self.id, &self.namespace)),
        ];
        for (field, checked) in fields {
            checked.map_err(|why| format!("[[engine]] `{}`: {field} {why}", self.alias))?;
        }
        if let Some(jump) = &self.jump {
            if jump.alias == self.alias {
                return Err(format!("[[engine]] `{}`: jump names the entry itself", self.alias));
            }
            jump.check()?;
        }
        Ok(())
    }

    /// Where the host is, as a refusal names it, through its jump where it takes one.
    pub fn place(&self) -> String {
        match &self.jump {
            None => format!("{}:{}", self.host, self.port),
            Some(jump) => format!("{}:{} through the jump `{}`", self.host, self.port, jump.alias),
        }
    }

    /// The exec command: grammar-ssh's line, through the login shell on Unix.
    pub fn command_line(&self) -> String {
        match self.platform {
            Platform::Windows => self.grammar_ssh_line(),
            Platform::Linux | Platform::Macos => {
                format!("exec /bin/sh -c 'exec \"$SHELL\" -l -c \"exec {}\"'", self.grammar_ssh_line())
            }
        }
    }

    /// grammar-ssh's own command line, as the login shell runs it.
    pub fn grammar_ssh_line(&self) -> String {
        let mut line = format!("{} --id {} --namespace {}", self.command, self.id, self.namespace);
        if let Some(workdir) = &self.workdir {
            line.push_str(" --workdir ");
            line.push_str(workdir);
        }
        line
    }

    /// The instance the session's hello expects.
    pub fn instance(&self) -> Result<Instance, TransportError> {
        Instance::new(&self.id, &self.namespace)
    }

    /// The host's pinned keys, each parsed.
    pub(crate) fn pinned_keys(&self) -> Result<Vec<keys::PublicKey>, String> {
        self.host_keys.iter().map(|line| pinned_key(line)).collect()
    }
}

/// An engine's alias: names joined by `/`, each a name, and never `local`.
pub fn check_alias(alias: &str) -> Result<(), String> {
    if alias.chars().count() > ALIAS_MAX_CHARS {
        return Err(format!(
            "[[engine]] alias `{alias}` passes {ALIAS_MAX_CHARS} characters"
        ));
    }
    if !alias.split('/').all(valid_name) {
        return Err(format!(
            "[[engine]] alias `{alias}` is not a name: names joined by `/`, each 1 to 64 letters, digits, `.`, `_`, or `-`, a letter or digit first"
        ));
    }
    if alias == LOCAL_HOST {
        return Err(format!("[[engine]] `{alias}`: alias `{LOCAL_HOST}` names the MCP's own engine"));
    }
    Ok(())
}

/// An address, `<host>[:<port>]`, as its host and port: 22 when it names none.
pub fn parse_address(address: &str) -> Result<(String, u16), String> {
    let (host, port) = match address.strip_prefix('[') {
        Some(rest) => {
            let (host, after) = rest
                .split_once(']')
                .ok_or_else(|| format!("`{address}` opens a bracket it never closes"))?;
            let port = match after {
                "" => None,
                _ => Some(
                    after
                        .strip_prefix(':')
                        .ok_or_else(|| format!("`{address}` holds `{after}` after its bracket, where `:<port>` goes"))?,
                ),
            };
            (host, port)
        }
        None => match address.rsplit_once(':') {
            Some((host, _)) if host.contains(':') => {
                return Err(format!("`{address}` holds more than one `:`; an IPv6 host goes in brackets, `[::1]:22`"));
            }
            Some((host, port)) => (host, Some(port)),
            None => (address, None),
        },
    };
    check_word(host).map_err(|why| format!("`{address}`'s host {why}"))?;
    let port = match port {
        None => SSH_PORT,
        Some(port) => port
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .ok_or_else(|| format!("`{address}`'s port `{port}` isn't a port from 1 to 65535"))?,
    };
    Ok((host.to_string(), port))
}

/// The name a known hosts file lists `host` under: `[host]:port` off port 22.
pub fn known_hosts_name(
    host: &str,
    port: u16,
) -> String {
    if port == SSH_PORT { host.to_string() } else { format!("[{host}]:{port}") }
}

/// A login's host, port, user, key, and pinned keys, a refusal naming its field.
fn check_login(
    alias: &str,
    host: &str,
    port: u16,
    user: &str,
    key: &str,
    host_keys: &[String],
) -> Result<(), String> {
    let fields = [
        ("host", check_word(host)),
        ("port", check_port(port)),
        ("user", check_word(user)),
        ("key", check_key(key)),
        ("known_hosts", check_host_keys(host_keys)),
    ];
    for (field, checked) in fields {
        checked.map_err(|why| format!("[[engine]] `{alias}`: {field} {why}"))?;
    }
    Ok(())
}

/// A host or a user: one word, not empty, with no whitespace or control character.
pub fn check_word(text: &str) -> Result<(), String> {
    let plain = !text.is_empty() && !text.chars().any(|c| c.is_whitespace() || c.is_control());
    if plain { Ok(()) } else { Err("is empty or holds a space or a control character".to_string()) }
}

/// A port: never zero.
pub fn check_port(port: u16) -> Result<(), String> {
    if port == 0 { Err("is zero".to_string()) } else { Ok(()) }
}

/// A key file's path, absolute once its caller has expanded it.
pub fn check_key(key: &str) -> Result<(), String> {
    if Path::new(key).is_absolute() {
        Ok(())
    } else {
        Err(format!("`{key}` isn't absolute once expanded"))
    }
}

/// Pinned host keys: one at least, each a plain OpenSSH key not signing with SHA-1.
pub fn check_host_keys(lines: &[String]) -> Result<(), String> {
    if lines.is_empty() {
        return Err("is empty: one pinned key is needed at least".to_string());
    }
    lines.iter().try_for_each(|line| pinned_key(line).map(|_| ()))
}

/// grammar-ssh's command: a name the login's `PATH` finds, or an absolute path.
pub fn check_command(
    command: &str,
    platform: Platform,
) -> Result<(), String> {
    if is_absolute(command, platform) {
        return shell_neutral_path(command, platform);
    }
    if valid_name(command) {
        Ok(())
    } else {
        Err(format!(
            "`{command}` is neither an absolute path nor a name the login's PATH finds: 1 to 64 letters, digits, `.`, `_`, or `-`, a letter or digit first"
        ))
    }
}

/// grammar-ssh's workdir: absolute, of characters login shells keep.
pub fn check_path(
    path: &str,
    platform: Platform,
) -> Result<(), String> {
    shell_neutral_path(path, platform)
}

/// Whether `path` is absolute on the platform: `/` on Unix, `X:\` on Windows.
fn is_absolute(
    path: &str,
    platform: Platform,
) -> bool {
    match platform {
        Platform::Windows => {
            let bytes = path.as_bytes();
            bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'\\'
        }
        Platform::Linux | Platform::Macos => path.starts_with('/'),
    }
}

/// grammar-ssh's id and namespace, each held to the instance names' rule.
pub fn check_instance(
    id: &str,
    namespace: &str,
) -> Result<(), String> {
    Instance::new(id, namespace).map(|_| ()).map_err(|e| e.message())
}

/// A pinned host key from its OpenSSH line, a plain key not signing with SHA-1.
pub(crate) fn pinned_key(line: &str) -> Result<keys::PublicKey, String> {
    let key = keys::PublicKey::from_openssh(line.trim())
        .map_err(|e| format!("holds a line that isn't an OpenSSH public key: {e}"))?;
    match key.algorithm() {
        keys::Algorithm::Dsa => Err("holds a DSA key, whose signatures are SHA-1's".to_string()),
        keys::Algorithm::Other(name) => Err(format!("holds a key of the unknown algorithm `{}`", name.as_str())),
        _ => Ok(key),
    }
}

/// `path` held absolute and to the characters every login shell reads as they are.
fn shell_neutral_path(
    path: &str,
    platform: Platform,
) -> Result<(), String> {
    match platform {
        Platform::Windows => {
            if !is_absolute(path, platform) {
                return Err(format!("`{path}` isn't a drive's absolute path, `C:\\...`"));
            }
            let neutral = path[2..]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'\\' | b'.' | b'_' | b'-'));
            if !neutral {
                return Err(format!(
                    "`{path}` holds a character a Windows shell might read: past its drive, only letters, digits, `\\`, `.`, `_`, and `-` pass"
                ));
            }
        }
        Platform::Linux | Platform::Macos => {
            if !is_absolute(path, platform) {
                return Err(format!("`{path}` isn't absolute"));
            }
            let neutral =
                path.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'_' | b'-' | b'+'));
            if !neutral {
                return Err(format!(
                    "`{path}` holds a character a login shell might read: only letters, digits, `/`, `.`, `_`, `-`, and `+` pass"
                ));
            }
        }
    }
    Ok(())
}
