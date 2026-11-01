//! `remotes.toml` beside the config: read at each use, edited live.
use crate::*;

/// The remotes file's name, beside the agent's config file.
pub const REMOTES_FILE: &str = "remotes.toml";

/// The address a listen binds when neither the call nor the entry names one.
const DEFAULT_LISTEN: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// The file layer: its `[[channel]]` entries.
#[derive(Debug, Default, ser::Deserialize)]
#[serde(deny_unknown_fields)]
struct RemotesToml {
    #[serde(default)]
    channel: Vec<RemoteToml>,
}

/// One `[[channel]]` entry (file layer): a peer by its alias.
#[derive(Debug, Clone, ser::Deserialize)]
#[serde(deny_unknown_fields)]
struct RemoteToml {
    /// The name the remote channel commands address this peer by.
    alias: Option<String>,
    /// The peer's known certificate to match; `remote_` is implied by the entry.
    cert: Option<String>,
    /// The `ip:port` a connect dials when the call names none.
    address: Option<String>,
    /// The `ip` or `ip:port` a listen binds when the call names none.
    listen: Option<String>,
}

/// One remote (model layer): its fields as the file writes them, each checked.
#[derive(Debug, Clone)]
pub(crate) struct RemoteEntry {
    pub alias: String,
    /// The peer's certificate, its path unexpanded.
    pub cert: String,
    /// The `ip:port` a connect dials by default.
    pub address: Option<String>,
    /// The `ip` or `ip:port` a listen binds by default.
    pub listen: Option<String>,
}

/// The fields a `remote channel set` gives, each replacing the entry's own.
#[derive(Debug, Default)]
pub(crate) struct RemoteChange {
    pub cert: Option<String>,
    pub address: Option<String>,
    pub listen: Option<String>,
}

impl RemoteEntry {
    /// Check one entry: its alias and certificate present, its addresses parsed.
    fn resolve(raw: RemoteToml) -> Result<Self, String> {
        let alias = raw
            .alias
            .filter(|alias| !alias.is_empty())
            .ok_or_else(|| "a [[channel]] entry is missing `alias`".to_string())?;
        let cert = raw
            .cert
            .filter(|cert| !cert.is_empty())
            .ok_or_else(|| format!("[[channel]] `{alias}` is missing `cert`"))?;
        if let Some(address) = &raw.address {
            dial_address(address).map_err(|e| format!("[[channel]] `{alias}`: {e}"))?;
        }
        if let Some(listen) = &raw.listen {
            bind_address(listen).map_err(|e| format!("[[channel]] `{alias}`: {e}"))?;
        }
        Ok(Self {
            alias,
            cert,
            address: raw.address,
            listen: raw.listen,
        })
    }

    /// The address a connect dials by default, none when the entry names none.
    pub(crate) fn dial(&self) -> Option<SocketAddr> {
        self.address.as_deref().and_then(|address| dial_address(address).ok())
    }

    /// The address a listen binds by default: the entry's, else loopback's.
    pub(crate) fn bind(&self) -> SocketAddr {
        self.listen
            .as_deref()
            .and_then(|listen| bind_address(listen).ok())
            .unwrap_or(SocketAddr::new(DEFAULT_LISTEN, 0))
    }

    /// The peer's certificate, its path expanded, an unset variable's stand-in used.
    pub(crate) fn peer_cert(
        &self,
        defaults: &[ExpandDefault],
    ) -> Result<PathBuf, String> {
        expand_path(Path::new(&self.cert), defaults)
    }
}

/// `~` and each `$VAR` in a path expanded, an unset variable read from `defaults`.
fn expand_path(
    raw: &Path,
    defaults: &[ExpandDefault],
) -> Result<PathBuf, String> {
    let lossy = raw.to_string_lossy();
    if !lossy.contains(['$', '~']) {
        return Ok(raw.to_path_buf());
    }
    shellexpand::full_with_context(lossy.as_ref(), home_dir, |var: &str| {
        if let Ok(value) = std::env::var(var)
            && !value.is_empty()
        {
            return Ok(Some(value));
        }
        match defaults.iter().find(|default| default.var == var) {
            Some(default) => Ok(Some(default.value.clone())),
            None => Err("environment variable unset or empty".to_string()),
        }
    })
    .map(|expanded| PathBuf::from(expanded.as_ref()))
    .map_err(|e| format!("expand {}: ${}: {}", raw.display(), e.var_name, e.cause))
}

/// `$HOME`, read per call: the tilde's source.
fn home_dir() -> Option<String> {
    std::env::var("HOME").ok()
}

/// An `ip:port` a connect dials.
pub(crate) fn dial_address(text: &str) -> Result<SocketAddr, String> {
    text.parse()
        .map_err(|e| format!("address `{text}` is not ip:port: {e}"))
}

/// An `ip` or `ip:port` a listen binds, a bare `ip` on a random port.
pub(crate) fn bind_address(text: &str) -> Result<SocketAddr, String> {
    text.parse::<SocketAddr>()
        .or_else(|_| text.parse::<IpAddr>().map(|ip| SocketAddr::new(ip, 0)))
        .map_err(|_| format!("listen `{text}` is neither ip nor ip:port"))
}

/// The remotes file's entries, none when the file is absent.
pub(crate) fn read_remotes(path: &Path) -> Result<Vec<RemoteEntry>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };
    parse_remotes(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// A remotes file's text, each entry checked and no alias twice.
pub(crate) fn parse_remotes(text: &str) -> Result<Vec<RemoteEntry>, String> {
    let file: RemotesToml = toml::from_str(text).map_err(|e| e.to_string())?;
    let mut entries: Vec<RemoteEntry> = Vec::new();
    for raw in file.channel {
        let entry = RemoteEntry::resolve(raw)?;
        if entries.iter().any(|seen| seen.alias == entry.alias) {
            return Err(format!("duplicate [[channel]] alias `{}`", entry.alias));
        }
        entries.push(entry);
    }
    Ok(entries)
}

/// `text` with `alias`'s entry made or changed by `change`, its comments kept.
pub(crate) fn set_entry(
    text: &str,
    alias: &str,
    change: &RemoteChange,
) -> Result<String, String> {
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| e.to_string())?;
    let tables = doc
        .entry("channel")
        .or_insert(toml_edit::Item::ArrayOfTables(toml_edit::ArrayOfTables::new()))
        .as_array_of_tables_mut()
        .ok_or_else(|| "`channel` is not an array of [[channel]] tables".to_string())?;
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
        .ok_or_else(|| format!("[[channel]] `{alias}` vanished mid-edit"))?;
    for (key, given) in [
        ("cert", &change.cert),
        ("address", &change.address),
        ("listen", &change.listen),
    ] {
        if let Some(given) = given {
            table[key] = toml_edit::value(given.as_str());
        }
    }
    let edited = doc.to_string();
    parse_remotes(&edited)?;
    Ok(edited)
}

/// `text` without `alias`'s entry, the rest of the file as it was.
pub(crate) fn unset_entry(
    text: &str,
    alias: &str,
) -> Result<String, String> {
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| e.to_string())?;
    let missing = || format!("no [[channel]] `{alias}`");
    let tables = doc
        .get_mut("channel")
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

/// Rewrite the remotes file by `edit`, atomically, under a lock on its directory.
pub(crate) fn edit_remotes(
    path: &Path,
    edit: impl FnOnce(&str) -> Result<String, String>,
) -> Result<(), String> {
    let dir = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .ok_or_else(|| format!("{} names no directory", path.display()))?;
    fs::create_dir_all(dir).map_err(|e| format!("make {}: {e}", dir.display()))?;
    let lock = fs::File::open(dir).map_err(|e| format!("open {}: {e}", dir.display()))?;
    lock.lock().map_err(|e| format!("lock {}: {e}", dir.display()))?;
    let (text, permissions) = match fs::read_to_string(path) {
        Ok(text) => (text, fs::metadata(path).ok().map(|meta| meta.permissions())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => (String::new(), None),
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };
    let edited = edit(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut staged = tempfile::NamedTempFile::new_in(dir)
        .map_err(|e| format!("stage in {}: {e}", dir.display()))?;
    staged
        .write_all(edited.as_bytes())
        .and_then(|()| staged.as_file().sync_all())
        .map_err(|e| format!("stage {}: {e}", path.display()))?;
    if let Some(permissions) = permissions {
        fs::set_permissions(staged.path(), permissions)
            .map_err(|e| format!("stage {}: {e}", path.display()))?;
    }
    staged
        .persist(path)
        .map_err(|e| format!("write {}: {}", path.display(), e.error))?;
    Ok(())
}
