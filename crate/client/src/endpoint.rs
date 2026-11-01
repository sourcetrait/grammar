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
        let refuse = |field: &str, why: String| Err(format!("[[engine]] `{}`: {field} {why}", self.alias));
        if !valid_name(&self.alias) {
            return Err(format!("[[engine]] alias `{}` is not a name: 1 to 64 letters, digits, `.`, `_`, or `-`, a letter or digit first", self.alias));
        }
        if self.alias == LOCAL_HOST {
            return refuse("alias", format!("`{LOCAL_HOST}` names the MCP's own engine"));
        }
        if !plain_word(&self.host) {
            return refuse("host", "is empty or holds a space or a control character".to_string());
        }
        if self.port == 0 {
            return refuse("port", "is zero".to_string());
        }
        if !plain_word(&self.user) {
            return refuse("user", "is empty or holds a space or a control character".to_string());
        }
        if !Path::new(&self.key).is_absolute() {
            return refuse("key", format!("`{}` isn't absolute once expanded", self.key));
        }
        if self.host_keys.is_empty() {
            return refuse("host_keys", "is empty: one pinned key is needed at least".to_string());
        }
        for line in &self.host_keys {
            if let Err(why) = pinned_key(line) {
                return refuse("host_keys", why);
            }
        }
        for (field, path) in [("command", &self.command), ("workdir", &self.workdir)] {
            if let Err(why) = shell_neutral_path(path) {
                return refuse(field, why);
            }
        }
        if let Err(e) = self.instance() {
            return refuse("id or namespace", e.message());
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

/// Whether `text` is one word: not empty, no whitespace, and no control character.
fn plain_word(text: &str) -> bool {
    !text.is_empty() && !text.chars().any(|c| c.is_whitespace() || c.is_control())
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
