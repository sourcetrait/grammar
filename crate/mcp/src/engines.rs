//! Remote engines' entries in the remotes file, `[[engine]]` beside `[[channel]]`.
use crate::*;

/// The kind a remote engine command's refused parameters report under.
pub(crate) const KIND_ENGINE_INVALID: &str = "remote::engine::invalid_params";

/// The kind a close of no open session reports under.
pub(crate) const KIND_ENGINE_NOT_OPEN: &str = "remote::engine::not_open";

/// The fields a session needs, in the file's order, each an entry may still lack.
const REQUIRED: [&str; 7] = ["host", "user", "key", "host_keys", "command", "id", "workdir"];

/// The file layer: its `[[engine]]` entries beside the peer's `[[channel]]` ones.
#[derive(Debug, Default, ser::Deserialize)]
#[serde(deny_unknown_fields)]
struct EnginesToml {
    #[serde(default)]
    engine: Vec<EngineToml>,
    #[serde(default)]
    #[allow(dead_code)]
    channel: Vec<toml::Table>,
}

/// One `[[engine]]` entry (file layer): any field but its alias may be absent.
#[derive(Debug, Clone, Default, ser::Deserialize)]
#[serde(deny_unknown_fields)]
struct EngineToml {
    alias: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    user: Option<String>,
    key: Option<String>,
    host_keys: Option<Vec<String>>,
    command: Option<String>,
    id: Option<String>,
    namespace: Option<String>,
    workdir: Option<String>,
}

/// A remote engine's entry (model layer): its fields as written, each checked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EngineEntry {
    pub alias: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    /// The private key file, its path unexpanded.
    pub key: Option<String>,
    pub host_keys: Option<Vec<String>>,
    pub command: Option<String>,
    pub id: Option<String>,
    pub namespace: Option<String>,
    pub workdir: Option<String>,
}

/// The fields a `set` or an `import` gives, each replacing the entry's own.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EngineChange {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub key: Option<String>,
    pub host_keys: Option<Vec<String>>,
    pub command: Option<String>,
    pub id: Option<String>,
    pub namespace: Option<String>,
    pub workdir: Option<String>,
}

impl EngineEntry {
    /// An entry of `alias` holding the fields `change` gives.
    pub(crate) fn of(
        alias: &str,
        change: &EngineChange,
    ) -> Self {
        let EngineChange {
            host,
            port,
            user,
            key,
            host_keys,
            command,
            id,
            namespace,
            workdir,
        } = change.clone();
        Self {
            alias: alias.to_string(),
            host,
            port,
            user,
            key,
            host_keys,
            command,
            id,
            namespace,
            workdir,
        }
    }

    /// One entry read: its alias a name, each field it gives held to that field's rule.
    fn resolve(raw: EngineToml) -> Result<Self, String> {
        let alias = raw
            .alias
            .filter(|alias| !alias.is_empty())
            .ok_or_else(|| "an [[engine]] entry is missing `alias`".to_string())?;
        let entry = Self {
            alias,
            host: raw.host,
            port: raw.port,
            user: raw.user,
            key: raw.key,
            host_keys: raw.host_keys,
            command: raw.command,
            id: raw.id,
            namespace: raw.namespace,
            workdir: raw.workdir,
        };
        entry.check_given()?;
        Ok(entry)
    }

    /// The alias a name, and each field the entry gives held to that field's rule.
    pub(crate) fn check_given(&self) -> Result<(), String> {
        check_alias(&self.alias)?;
        let refuse = |field: &str| {
            let alias = self.alias.clone();
            let field = field.to_string();
            move |why: String| format!("[[engine]] `{alias}`: {field} {why}")
        };
        if let Some(host) = &self.host {
            check_word(host).map_err(refuse("host"))?;
        }
        if let Some(port) = self.port {
            check_port(port).map_err(refuse("port"))?;
        }
        if let Some(user) = &self.user {
            check_word(user).map_err(refuse("user"))?;
        }
        if let Some(key) = &self.key {
            check_key(&expanded_key(key)?).map_err(refuse("key"))?;
        }
        if let Some(lines) = &self.host_keys {
            check_host_keys(lines).map_err(refuse("host_keys"))?;
        }
        if let Some(command) = &self.command {
            check_path(command).map_err(refuse("command"))?;
        }
        if let Some(workdir) = &self.workdir {
            check_path(workdir).map_err(refuse("workdir"))?;
        }
        if self.id.is_some() || self.namespace.is_some() {
            let id = self.id.as_deref().unwrap_or(ENGINE_NAMESPACE);
            let namespace = self.namespace.as_deref().unwrap_or(ENGINE_NAMESPACE);
            check_instance(id, namespace).map_err(refuse("id or namespace"))?;
        }
        Ok(())
    }

    /// The fields a session needs that the entry lacks, in the file's order.
    pub(crate) fn missing(&self) -> Vec<String> {
        let given = [
            self.host.is_some(),
            self.user.is_some(),
            self.key.is_some(),
            self.host_keys.is_some(),
            self.command.is_some(),
            self.id.is_some(),
            self.workdir.is_some(),
        ];
        REQUIRED
            .iter()
            .zip(given)
            .filter(|(_, given)| !given)
            .map(|(field, _)| field.to_string())
            .collect()
    }

    /// The endpoint a session opens on: every field present, the key expanded, checked.
    pub(crate) fn endpoint(&self) -> Result<Endpoint, String> {
        let missing = self.missing();
        if !missing.is_empty() {
            let fields: Vec<String> = missing.iter().map(|field| format!("`{field}`")).collect();
            return Err(format!(
                "remote engine `{}` is missing {}; give them with `interface remote engine set` or `import`",
                self.alias,
                fields.join(", "),
            ));
        }
        let text = |field: &Option<String>| field.clone().unwrap_or_default();
        let endpoint = Endpoint {
            alias: self.alias.clone(),
            host: text(&self.host),
            port: self.port.unwrap_or(SSH_PORT),
            user: text(&self.user),
            key: expanded_key(&text(&self.key))?,
            host_keys: self.host_keys.clone().unwrap_or_default(),
            command: text(&self.command),
            id: text(&self.id),
            namespace: self.namespace.clone().unwrap_or_else(|| ENGINE_NAMESPACE.to_string()),
            workdir: text(&self.workdir),
        };
        endpoint.check()?;
        Ok(endpoint)
    }
}

/// An entry's key file expanded as the config's paths are.
fn expanded_key(key: &str) -> Result<String, String> {
    expand_path(Path::new(key)).map(|path| path.to_string_lossy().into_owned())
}

/// The remotes file's engines, none when the file is absent.
pub(crate) fn read_engines(path: &Path) -> Result<Vec<EngineEntry>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };
    parse_engines(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// A remotes file's text, each engine checked and no alias twice.
pub(crate) fn parse_engines(text: &str) -> Result<Vec<EngineEntry>, String> {
    let file: EnginesToml = toml::from_str(text).map_err(|e| e.to_string())?;
    let mut entries: Vec<EngineEntry> = Vec::new();
    for raw in file.engine {
        let entry = EngineEntry::resolve(raw)?;
        if entries.iter().any(|seen| seen.alias == entry.alias) {
            return Err(format!("duplicate [[engine]] alias `{}`", entry.alias));
        }
        entries.push(entry);
    }
    Ok(entries)
}

/// The endpoint the remotes file's entry for `alias` opens a session on.
pub(crate) fn engine_endpoint(
    path: &Path,
    alias: &str,
) -> Result<Endpoint, String> {
    read_engines(path)?
        .into_iter()
        .find(|entry| entry.alias == alias)
        .ok_or_else(|| format!("no remote engine `{alias}` in the remotes file"))?
        .endpoint()
}

/// `text` with `alias`'s engine made or changed by `change`, its comments kept.
pub(crate) fn set_engine(
    text: &str,
    alias: &str,
    change: &EngineChange,
) -> Result<String, String> {
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| e.to_string())?;
    let tables = doc
        .entry("engine")
        .or_insert(toml_edit::Item::ArrayOfTables(toml_edit::ArrayOfTables::new()))
        .as_array_of_tables_mut()
        .ok_or_else(|| "`engine` is not an array of [[engine]] tables".to_string())?;
    let found = tables.iter().position(|table| alias_of(table) == Some(alias));
    let index = match found {
        Some(index) => index,
        None => {
            let mut table = toml_edit::Table::new();
            table["alias"] = toml_edit::value(alias);
            tables.push(table);
            tables.len() - 1
        }
    };
    let table = tables
        .get_mut(index)
        .ok_or_else(|| format!("[[engine]] `{alias}` vanished mid-edit"))?;
    let text_value = |field: &Option<String>| field.as_deref().map(toml_edit::value);
    let given = [
        ("host", text_value(&change.host)),
        ("port", change.port.map(|port| toml_edit::value(i64::from(port)))),
        ("user", text_value(&change.user)),
        ("key", text_value(&change.key)),
        (
            "host_keys",
            change
                .host_keys
                .as_ref()
                .map(|keys| toml_edit::value(keys.iter().map(String::as_str).collect::<toml_edit::Array>())),
        ),
        ("command", text_value(&change.command)),
        ("id", text_value(&change.id)),
        ("namespace", text_value(&change.namespace)),
        ("workdir", text_value(&change.workdir)),
    ];
    for (key, value) in given {
        if let Some(value) = value {
            table[key] = value;
        }
    }
    let edited = doc.to_string();
    parse_engines(&edited)?;
    Ok(edited)
}

/// `text` without `alias`'s engine, the rest of the file as it was.
pub(crate) fn unset_engine(
    text: &str,
    alias: &str,
) -> Result<String, String> {
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| e.to_string())?;
    let missing = || format!("no [[engine]] `{alias}`");
    let tables = doc
        .get_mut("engine")
        .and_then(|item| item.as_array_of_tables_mut())
        .ok_or_else(missing)?;
    let index = tables
        .iter()
        .position(|table| alias_of(table) == Some(alias))
        .ok_or_else(missing)?;
    tables.remove(index);
    Ok(doc.to_string())
}

/// A table's `alias`, when it holds a string one.
fn alias_of(table: &toml_edit::Table) -> Option<&str> {
    table.get("alias").and_then(|item| item.as_str())
}

/// What `ssh -G` resolves for a host that an engine's entry takes.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct SshResolved {
    pub hostname: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub identity_files: Vec<String>,
    pub known_hosts_files: Vec<String>,
    pub host_key_alias: Option<String>,
}

/// `ssh -G`'s output read, a `keyword value` a line.
pub(crate) fn parse_ssh_g(output: &str) -> SshResolved {
    let mut resolved = SshResolved::default();
    for line in output.lines() {
        let Some((keyword, value)) = line.trim().split_once(' ') else {
            continue;
        };
        let value = value.trim();
        match keyword {
            "hostname" => resolved.hostname = Some(value.to_string()),
            "port" => resolved.port = value.parse().ok(),
            "user" => resolved.user = Some(value.to_string()),
            "identityfile" => resolved.identity_files.push(value.to_string()),
            "userknownhostsfile" | "globalknownhostsfile" => {
                resolved.known_hosts_files.extend(value.split_whitespace().map(str::to_string));
            }
            "hostkeyalias" if value != "none" => resolved.host_key_alias = Some(value.to_string()),
            _ => {}
        }
    }
    resolved
}

/// The keys `ssh-keygen -F`'s output lists, each `type key` once, markers skipped.
pub(crate) fn parse_known_keys(output: &str) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for line in output.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') || line.starts_with('@') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let (Some(_names), Some(kind), Some(data)) = (fields.next(), fields.next(), fields.next()) else {
            continue;
        };
        let key = format!("{kind} {data}");
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys
}

/// The name a known hosts file lists `host` under: `[host]:port` off port 22.
pub(crate) fn known_hosts_name(
    host: &str,
    port: u16,
) -> String {
    if port == SSH_PORT { host.to_string() } else { format!("[{host}]:{port}") }
}

/// The fields an ssh configuration's host gives an entry, by OpenSSH's own client.
///
/// `ssh -G` resolves the host, its includes and matches applied, and `ssh-keygen -F`
/// finds its keys in the known hosts files it names.
pub(crate) fn import_ssh(
    from: &str,
    ssh_config: Option<&str>,
) -> Result<EngineChange, String> {
    let mut resolve = process::Command::new("ssh");
    if let Some(config) = ssh_config {
        resolve.arg("-F").arg(expand_path(Path::new(config))?);
    }
    let output = resolve
        .arg("-G")
        .arg(from)
        .stdin(process::Stdio::null())
        .output()
        .map_err(|e| format!("`ssh -G {from}`: {e}; an import reads OpenSSH's client"))?;
    if !output.status.success() {
        return Err(format!("`ssh -G {from}`: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    let resolved = parse_ssh_g(&String::from_utf8_lossy(&output.stdout));
    let host = resolved
        .hostname
        .clone()
        .ok_or_else(|| format!("ssh resolves no host name for `{from}`"))?;
    let port = resolved.port.unwrap_or(SSH_PORT);
    let key = resolved
        .identity_files
        .iter()
        .find(|file| expand_path(Path::new(file)).is_ok_and(|path| path.is_file()))
        .cloned()
        .ok_or_else(|| {
            format!("no identity file ssh names for `{from}` exists: {}", resolved.identity_files.join(", "))
        })?;
    let lookup = resolved
        .host_key_alias
        .clone()
        .unwrap_or_else(|| known_hosts_name(&host, port));
    let mut host_keys: Vec<String> = Vec::new();
    for file in &resolved.known_hosts_files {
        let Ok(path) = expand_path(Path::new(file)) else {
            continue;
        };
        if !path.is_file() {
            continue;
        }
        let found = process::Command::new("ssh-keygen")
            .arg("-F")
            .arg(&lookup)
            .arg("-f")
            .arg(&path)
            .stdin(process::Stdio::null())
            .output()
            .map_err(|e| format!("`ssh-keygen -F {lookup}`: {e}"))?;
        for key in parse_known_keys(&String::from_utf8_lossy(&found.stdout)) {
            if !host_keys.contains(&key) {
                host_keys.push(key);
            }
        }
    }
    if host_keys.is_empty() {
        return Err(format!(
            "no known host key for `{lookup}` in {}; connect once with ssh to learn it, or give `--host-keys` to `set`",
            resolved.known_hosts_files.join(", "),
        ));
    }
    Ok(EngineChange {
        host: Some(host),
        port: Some(port),
        user: resolved.user,
        key: Some(key),
        host_keys: Some(host_keys),
        ..EngineChange::default()
    })
}
