//! A remote engine's entry: its host, how the host is trusted, and what runs there.
use crate::*;

/// The SSH port an entry takes when it names none.
pub const SSH_PORT: u16 = 22;

/// The namespace an entry takes when it names none.
pub const ENGINE_NAMESPACE: &str = "default";

/// A remote engine as the MCP's config names it, each field checked by `check`.
#[cereal::derived(Data, Eq)]
pub struct Endpoint {
    /// The name a tool's `host` gives the engine.
    pub alias: String,
    /// The host's name or address.
    pub host: String,
    pub port: u16,
    /// The account the session logs in as.
    pub user: String,
    /// The private key file the session authenticates with, on the MCP's host.
    pub key: String,
    /// The host's public keys as OpenSSH lines, one of which it must present.
    pub host_keys: Vec<String>,
    /// The worker's absolute path on the host.
    pub command: String,
    /// The worker's id.
    pub id: String,
    /// The worker's namespace.
    pub namespace: String,
    /// The worker's working directory on the host.
    pub workdir: String,
}

impl Endpoint {
    /// The entry held to its rules, each refusal naming the field it is about.
    pub fn check(&self) -> Result<(), String> {
        check_alias(&self.alias)?;
        let fields = [
            ("host", check_word(&self.host)),
            ("port", check_port(self.port)),
            ("user", check_word(&self.user)),
            ("key", check_key(&self.key)),
            ("host_keys", check_host_keys(&self.host_keys)),
            ("command", check_path(&self.command)),
            ("workdir", check_path(&self.workdir)),
            ("id or namespace", check_instance(&self.id, &self.namespace)),
        ];
        for (field, checked) in fields {
            checked.map_err(|why| format!("[[engine]] `{}`: {field} {why}", self.alias))?;
        }
        Ok(())
    }

    /// The worker's command line, which the host's login shell runs as it stands.
    pub fn command_line(&self) -> String {
        format!(
            "{} serve --id {} --namespace {} --workdir {}",
            self.command, self.id, self.namespace, self.workdir
        )
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

/// An engine's alias held to the name rule and kept from `local`, refused whole.
pub fn check_alias(alias: &str) -> Result<(), String> {
    if !valid_name(alias) {
        return Err(format!(
            "[[engine]] alias `{alias}` is not a name: 1 to 64 letters, digits, `.`, `_`, or `-`, a letter or digit first"
        ));
    }
    if alias == LOCAL_HOST {
        return Err(format!("[[engine]] `{alias}`: alias `{LOCAL_HOST}` names the MCP's own engine"));
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

/// The worker's command or workdir: absolute, of characters login shells keep.
pub fn check_path(path: &str) -> Result<(), String> {
    shell_neutral_path(path)
}

/// The worker's id and namespace, each held to the instance names' rule.
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
fn shell_neutral_path(path: &str) -> Result<(), String> {
    if !path.starts_with('/') {
        return Err(format!("`{path}` isn't absolute"));
    }
    let neutral = path.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'_' | b'-' | b'+'));
    if !neutral {
        return Err(format!(
            "`{path}` holds a character a login shell might read: only letters, digits, `/`, `.`, `_`, `-`, and `+` pass"
        ));
    }
    Ok(())
}
