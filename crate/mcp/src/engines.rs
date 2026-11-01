//! Remote engines' entries in the remotes file, `[[engine]]` beside `[[channel]]`.
use crate::*;

/// The kind a remote engine command's refused parameters report under.
pub(crate) const KIND_ENGINE_INVALID: &str = "remote::engine::invalid_params";

/// The kind a close of no open session reports under.
pub(crate) const KIND_ENGINE_NOT_OPEN: &str = "remote::engine::not_open";

/// The fields a session needs, in the file's order, each an entry may still lack.
const REQUIRED: [&str; 4] = ["address", "user", "key", "known_hosts"];

/// The fields an older entry wrote, each with the field that took its place.
const RETIRED: [(&str, &str); 3] = [("host", "address"), ("port", "address"), ("host_keys", "known_hosts")];

/// A stand-in for an absent id or namespace while the given one is checked.
const NAME_STAND_IN: &str = "default";

/// The first line of an import's known hosts file, which a rewrite looks for.
const IMPORT_MARK: &str = "# grammar: written by `interface remote engine import`";

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
    address: Option<String>,
    user: Option<String>,
    key: Option<String>,
    known_hosts: Option<KnownHosts>,
    command: Option<String>,
    id: Option<String>,
    namespace: Option<String>,
    workdir: Option<String>,
    /// A retired field, read only to be refused with what took its place.
    host: Option<toml::Value>,
    /// A retired field, read only to be refused with what took its place.
    port: Option<toml::Value>,
    /// A retired field, read only to be refused with what took its place.
    host_keys: Option<toml::Value>,
}

/// An entry's known hosts: the host's keys inline, or a file in OpenSSH's format.
#[derive(Debug, Clone, PartialEq, Eq, ser::Deserialize, ser::Serialize)]
#[serde(untagged)]
pub(crate) enum KnownHosts {
    /// The host's keys as OpenSSH lines.
    Keys(Vec<String>),
    /// A known_hosts file's path, a relative one beside the remotes file.
    File(String),
}

/// A remote engine's entry (model layer): its fields as written, each checked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EngineEntry {
    pub alias: String,
    /// The host as `<host>[:<port>]`.
    pub address: Option<String>,
    pub user: Option<String>,
    /// The private key file, its path unexpanded.
    pub key: Option<String>,
    pub known_hosts: Option<KnownHosts>,
    pub command: Option<String>,
    pub id: Option<String>,
    pub namespace: Option<String>,
    pub workdir: Option<String>,
}

/// The fields a `set` or an `import` gives, each replacing the entry's own.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EngineChange {
    pub address: Option<String>,
    pub user: Option<String>,
    pub key: Option<String>,
    pub known_hosts: Option<KnownHosts>,
    pub command: Option<String>,
    pub id: Option<String>,
    pub namespace: Option<String>,
    pub workdir: Option<String>,
}

/// What an entry's absent fields take, and where its relative paths are read from.
#[derive(Debug, Clone)]
pub(crate) struct EngineDefaults {
    /// The MCP's own id.
    pub id: String,
    /// The MCP's own namespace.
    pub namespace: String,
    /// The remotes file's directory.
    pub dir: PathBuf,
}

impl EngineDefaults {
    /// The defaults `config` gives: its id, namespace, and remotes directory.
    pub(crate) fn of(config: &Config) -> Self {
        Self {
            id: config.id.clone(),
            namespace: config.namespace.clone(),
            dir: config.remotes_file.parent().map(Path::to_path_buf).unwrap_or_default(),
        }
    }
}

impl EngineEntry {
    /// An entry of `alias` holding the fields `change` gives.
    pub(crate) fn of(
        alias: &str,
        change: &EngineChange,
    ) -> Self {
        let EngineChange {
            address,
            user,
            key,
            known_hosts,
            command,
            id,
            namespace,
            workdir,
        } = change.clone();
        Self {
            alias: alias.to_string(),
            address,
            user,
            key,
            known_hosts,
            command,
            id,
            namespace,
            workdir,
        }
    }

    /// One entry read: its alias a name, no retired field, each field held to its rule.
    fn resolve(raw: EngineToml) -> Result<Self, String> {
        let alias = raw
            .alias
            .filter(|alias| !alias.is_empty())
            .ok_or_else(|| "an [[engine]] entry is missing `alias`".to_string())?;
        let retired = [raw.host.is_some(), raw.port.is_some(), raw.host_keys.is_some()];
        if let Some(((field, successor), _)) = RETIRED.iter().zip(retired).find(|(_, given)| *given) {
            return Err(format!(
                "[[engine]] `{alias}`: `{field}` is retired; `{successor}` takes its place"
            ));
        }
        let entry = Self {
            alias,
            address: raw.address,
            user: raw.user,
            key: raw.key,
            known_hosts: raw.known_hosts,
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
        if let Some(address) = &self.address {
            parse_address(address).map_err(refuse("address"))?;
        }
        if let Some(user) = &self.user {
            check_word(user).map_err(refuse("user"))?;
        }
        if let Some(key) = &self.key {
            check_key(&expanded_key(key)?).map_err(refuse("key"))?;
        }
        match &self.known_hosts {
            Some(KnownHosts::Keys(lines)) => check_host_keys(lines).map_err(refuse("known_hosts"))?,
            Some(KnownHosts::File(path)) => check_file(path).map_err(refuse("known_hosts"))?,
            None => {}
        }
        if let Some(command) = &self.command {
            check_command(command).map_err(refuse("command"))?;
        }
        if let Some(workdir) = &self.workdir {
            check_path(workdir).map_err(refuse("workdir"))?;
        }
        if self.id.is_some() || self.namespace.is_some() {
            let id = self.id.as_deref().unwrap_or(NAME_STAND_IN);
            let namespace = self.namespace.as_deref().unwrap_or(NAME_STAND_IN);
            check_instance(id, namespace).map_err(refuse("id or namespace"))?;
        }
        Ok(())
    }

    /// The fields a session needs that the entry lacks, in the file's order.
    pub(crate) fn missing(&self) -> Vec<String> {
        let given = [
            self.address.is_some(),
            self.user.is_some(),
            self.key.is_some(),
            self.known_hosts.is_some(),
        ];
        REQUIRED
            .iter()
            .zip(given)
            .filter(|(_, given)| !given)
            .map(|(field, _)| field.to_string())
            .collect()
    }

    /// The endpoint a session opens on: every field resolved, defaults taken, checked.
    pub(crate) fn endpoint(
        &self,
        defaults: &EngineDefaults,
    ) -> Result<Endpoint, String> {
        let missing = self.missing();
        if !missing.is_empty() {
            let fields: Vec<String> = missing.iter().map(|field| format!("`{field}`")).collect();
            return Err(format!(
                "remote engine `{}` is missing {}; give them with `interface remote engine set` or `import`",
                self.alias,
                fields.join(", "),
            ));
        }
        let refuse = |field: &str, why: String| format!("[[engine]] `{}`: {field} {why}", self.alias);
        let text = |field: &Option<String>| field.clone().unwrap_or_default();
        let (host, port) = parse_address(&text(&self.address)).map_err(|why| refuse("address", why))?;
        let host_keys = match &self.known_hosts {
            Some(KnownHosts::Keys(lines)) => lines.clone(),
            Some(KnownHosts::File(path)) => {
                file_keys(path, &host, port, defaults).map_err(|why| refuse("known_hosts", why))?
            }
            None => Vec::new(),
        };
        let endpoint = Endpoint {
            alias: self.alias.clone(),
            host,
            port,
            user: text(&self.user),
            key: expanded_key(&text(&self.key))?,
            host_keys,
            command: self.command.clone().unwrap_or_else(|| WORKER_COMMAND.to_string()),
            id: self.id.clone().unwrap_or_else(|| defaults.id.clone()),
            namespace: self.namespace.clone().unwrap_or_else(|| defaults.namespace.clone()),
            workdir: self.workdir.clone(),
        };
        endpoint.check()?;
        Ok(endpoint)
    }
}

/// An entry's key file expanded as the config's paths are.
fn expanded_key(key: &str) -> Result<String, String> {
    expand_path(Path::new(key)).map(|path| path.to_string_lossy().into_owned())
}

/// A known hosts file's path as written: not empty, no control character.
fn check_file(path: &str) -> Result<(), String> {
    if path.trim().is_empty() || path.chars().any(char::is_control) {
        Err("names a file that is empty or holds a control character".to_string())
    } else {
        Ok(())
    }
}

/// A known hosts path expanded, a relative one read beside the remotes file.
fn known_hosts_path(
    path: &str,
    defaults: &EngineDefaults,
) -> Result<PathBuf, String> {
    let expanded = expand_path(Path::new(path))?;
    Ok(if expanded.is_absolute() { expanded } else { defaults.dir.join(expanded) })
}

/// The keys a known hosts file holds for `host` at `port`, one at least.
fn file_keys(
    path: &str,
    host: &str,
    port: u16,
    defaults: &EngineDefaults,
) -> Result<Vec<String>, String> {
    let file = known_hosts_path(path, defaults)?;
    let text = fs::read_to_string(&file).map_err(|e| format!("`{}` couldn't be read: {e}", file.display()))?;
    let keys = known_host_keys(&text, host, port).map_err(|e| format!("`{}`: {e}", file.display()))?;
    if keys.is_empty() {
        return Err(format!("`{}` holds no key for `{}`", file.display(), known_hosts_name(host, port)));
    }
    Ok(keys)
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
    defaults: &EngineDefaults,
) -> Result<Endpoint, String> {
    read_engines(path)?
        .into_iter()
        .find(|entry| entry.alias == alias)
        .ok_or_else(|| format!("no remote engine `{alias}` in the remotes file"))?
        .endpoint(defaults)
}

/// `text` with `alias`'s engine made or changed by `change`, its comments kept.
///
/// A field given drops the retired fields it replaces, so an older entry moves to the
/// new ones by a `set` of them.
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
    let known_hosts = change.known_hosts.as_ref().map(|known| match known {
        KnownHosts::Keys(keys) => toml_edit::value(keys.iter().map(String::as_str).collect::<toml_edit::Array>()),
        KnownHosts::File(path) => toml_edit::value(path.as_str()),
    });
    let given = [
        ("address", text_value(&change.address)),
        ("user", text_value(&change.user)),
        ("key", text_value(&change.key)),
        ("known_hosts", known_hosts),
        ("command", text_value(&change.command)),
        ("id", text_value(&change.id)),
        ("namespace", text_value(&change.namespace)),
        ("workdir", text_value(&change.workdir)),
    ];
    for (key, value) in given {
        if let Some(value) = value {
            table[key] = value;
            for (retired, successor) in RETIRED {
                if successor == key {
                    table.remove(retired);
                }
            }
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

/// An address for `host` and `port`: no port at 22, an IPv6 host bracketed.
pub(crate) fn address_of(
    host: &str,
    port: u16,
) -> String {
    let host = if host.contains(':') { format!("[{host}]") } else { host.to_string() };
    if port == SSH_PORT { host } else { format!("{host}:{port}") }
}

/// What an import takes from an ssh configuration's host: its fields and its keys.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct SshImport {
    /// The entry's address, user, and key; its known hosts are the file the keys go to.
    pub change: EngineChange,
    /// The name the keys are listed under, `[host]:port` off port 22.
    pub name: String,
    /// The host's keys, each an OpenSSH line.
    pub keys: Vec<String>,
}

/// The fields an ssh configuration's host gives an entry, by OpenSSH's own client.
///
/// `ssh -G` resolves the host, its includes and matches applied, and `ssh-keygen -F`
/// finds its keys in the known hosts files it names.
pub(crate) fn import_ssh(
    from: &str,
    ssh_config: Option<&str>,
) -> Result<SshImport, String> {
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
    let mut keys: Vec<String> = Vec::new();
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
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
    }
    if keys.is_empty() {
        return Err(format!(
            "no known host key for `{lookup}` in {}; connect once with ssh to learn it, or give `--known-hosts` to `set`",
            resolved.known_hosts_files.join(", "),
        ));
    }
    Ok(SshImport {
        change: EngineChange {
            address: Some(address_of(&host, port)),
            user: resolved.user,
            key: Some(key),
            ..EngineChange::default()
        },
        name: known_hosts_name(&host, port),
        keys,
    })
}

/// The known hosts file an import writes for `alias`, beside the remotes file.
pub(crate) fn import_known_hosts(alias: &str) -> String {
    format!("remote/{alias}/known_hosts")
}

/// `keys` as `alias`'s known hosts file in `dir`, never over another's file.
pub(crate) fn write_known_hosts(
    dir: &Path,
    alias: &str,
    name: &str,
    keys: &[String],
) -> Result<String, String> {
    check_alias(alias)?;
    let relative = import_known_hosts(alias);
    let file = dir.join(&relative);
    match fs::read_to_string(&file) {
        Ok(text) if !text.starts_with(IMPORT_MARK) => {
            return Err(format!(
                "`{}` exists and isn't an import's; move it, or name it with `set --known-hosts-file`",
                file.display()
            ));
        }
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("`{}` couldn't be read: {e}", file.display())),
    }
    let parent = file
        .parent()
        .ok_or_else(|| format!("`{}` has no directory", file.display()))?;
    fs::create_dir_all(parent).map_err(|e| format!("`{}` couldn't be made: {e}", parent.display()))?;
    let mut text = format!("{IMPORT_MARK} for remote engine `{alias}`; an import rewrites it.\n");
    for key in keys {
        text.push_str(&format!("{name} {key}\n"));
    }
    let staged = parent.join(".known_hosts.import");
    fs::write(&staged, text).map_err(|e| format!("`{}` couldn't be written: {e}", staged.display()))?;
    fs::rename(&staged, &file).map_err(|e| format!("`{}` couldn't be written: {e}", file.display()))?;
    Ok(relative)
}
