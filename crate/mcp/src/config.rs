//! The runtime configuration, split format-layer from model-layer.
use crate::*;

/// The embedded base every load merges onto.
const DEFAULTS_CONFIG: &str = include_str!("../../../include/config/grammar_mcp.toml");

/// The namespace when `--namespace` is not given.
pub(crate) const DEFAULT_NAMESPACE: &str = "default";

/// The namespace `--test` defaults to.
pub(crate) const TEST_NAMESPACE: &str = "test";

/// Lowest port the channel hub may be pinned to.
const MIN_CHANNEL_PORT: u16 = 1024;

/// The agent's config file, under the directory the host starts in.
pub(crate) const AGENT_CONFIG_FILE: &str = ".grammar/config.toml";

/// The file shape: the embedded base, and the agent's own file over it.
#[derive(Debug, Clone, Default, ser::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfigToml {
    pub channel: Option<ChannelConfigToml>,
    pub supervisor: Option<SupervisorConfigToml>,
    pub identity: Option<IdentityConfigToml>,
}

/// The `[identity]` table (file layer): the host's own key pair.
#[derive(Debug, Clone, Default, ser::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IdentityConfigToml {
    /// The host's certificate, PEM.
    pub cert: Option<String>,
    /// The private key paired with `cert`, PEM.
    pub key: Option<String>,
}

/// The `[supervisor]` table: when a resource level is worth one warning.
#[derive(Debug, Clone, Default, ser::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SupervisorConfigToml {
    /// Fraction of ALL cores the host's own CPU must reach.
    pub cpu_warn_fraction: Option<f64>,
    /// Fraction of total system RAM the host's own RSS must reach.
    pub ram_warn_fraction: Option<f64>,
    /// Warn once GPU memory FREE falls below this many MiB.
    pub vram_warn_headroom_mib: Option<u64>,
    /// Fraction of a watched filesystem's capacity.
    pub disk_warn_fraction: Option<f64>,
}

/// The `[channel]` table: the wss port and the spam policy.
#[derive(Debug, Clone, Default, ser::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChannelConfigToml {
    /// Omit for a kernel-assigned port, which is the default.
    pub port: Option<u16>,
    /// Windows are whole seconds here and on `interface channel config`.
    pub spam_warn_window_secs: Option<u64>,
    pub spam_warn_rate: Option<u32>,
    pub spam_error_window_secs: Option<u64>,
    pub spam_error_rate: Option<u32>,
}

#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub id: String,
    pub namespace: String,
    pub work_dir: PathBuf,
    pub deny: DenySet,
    /// Was this host launched with `--test`?
    pub test: bool,
    pub channel: ChannelConfig,
    pub supervisor: SupervisorConfig,
    /// The host's own key pair, from `[identity]`; none when the file names none.
    pub identity: Option<IdentityConfig>,
    /// The remotes file beside the config: its channels and engines, read at each use.
    pub remotes_file: PathBuf,
}

/// The host's own key pair (model layer), each path expanded.
#[derive(Debug, Clone)]
pub(crate) struct IdentityConfig {
    pub cert: PathBuf,
    pub key: PathBuf,
}

/// The channel hub's operating parameters.
#[derive(Debug, Clone)]
pub(crate) struct ChannelConfig {
    /// None means kernel-assigned, which is the default.
    pub port: Option<u16>,
    /// The starting spam policy.
    pub spam: SpamThresholds,
}

/// Merge one `[channel]` table over the embedded base, then the code defaults.
fn merged_channel(
    user: Option<ChannelConfigToml>,
    base: Option<ChannelConfigToml>,
) -> Result<ChannelConfig, String> {
    let user = user.unwrap_or_default();
    let base = base.unwrap_or_default();
    let port = user.port.or(base.port);
    if let Some(port) = port
        && port < MIN_CHANNEL_PORT
    {
        return Err(format!(
            "channel.port must be {MIN_CHANNEL_PORT}-65535 (below that is privileged); \
             omit it for a kernel-assigned port",
        ));
    }
    let spam = SpamThresholds {
        warn_window: secs_field("spam_warn_window_secs", user.spam_warn_window_secs, base.spam_warn_window_secs)?,
        warn_rate: rate_field("spam_warn_rate", user.spam_warn_rate, base.spam_warn_rate)?,
        error_window: secs_field("spam_error_window_secs", user.spam_error_window_secs, base.spam_error_window_secs)?,
        error_rate: rate_field("spam_error_rate", user.spam_error_rate, base.spam_error_rate)?,
    };
    Ok(ChannelConfig { port, spam })
}

/// The `[identity]` table resolved, both its files required and expanded.
fn resolved_identity(file: Option<IdentityConfigToml>) -> Result<Option<IdentityConfig>, String> {
    let Some(file) = file else {
        return Ok(None);
    };
    let missing = |field: &str| format!("[identity] is missing `{field}`");
    let cert = file.cert.ok_or_else(|| missing("cert"))?;
    let key = file.key.ok_or_else(|| missing("key"))?;
    Ok(Some(IdentityConfig {
        cert: expand_path(Path::new(&cert))?,
        key: expand_path(Path::new(&key))?,
    }))
}

fn merged_supervisor(
    user: Option<SupervisorConfigToml>,
    base: Option<SupervisorConfigToml>,
) -> Result<SupervisorConfig, String> {
    let user = user.unwrap_or_default();
    let base = base.unwrap_or_default();
    Ok(SupervisorConfig {
        cpu_warn_fraction: fraction_field(
            "cpu_warn_fraction",
            user.cpu_warn_fraction.or(base.cpu_warn_fraction),
        )?,
        ram_warn_fraction: fraction_field(
            "ram_warn_fraction",
            user.ram_warn_fraction.or(base.ram_warn_fraction),
        )?,
        vram_warn_headroom_mib: user
            .vram_warn_headroom_mib
            .or(base.vram_warn_headroom_mib)
            .ok_or_else(|| "the embedded defaults carry no supervisor.vram_warn_headroom_mib".to_string())?,
        disk_warn_fraction: fraction_field(
            "disk_warn_fraction",
            user.disk_warn_fraction.or(base.disk_warn_fraction),
        )?,
    })
}

/// A supervisor fraction the merged layers carry, held to the operator's range.
fn fraction_field(
    name: &str,
    value: Option<f64>,
) -> Result<f64, String> {
    match value {
        None => Err(format!("the embedded defaults carry no supervisor.{name}")),
        Some(f) => check_fraction(name, f),
    }
}

/// A window, in whole seconds.
fn secs_field(
    name: &str,
    user: Option<u64>,
    base: Option<u64>,
) -> Result<tk::TkDuration, String> {
    match user.or(base) {
        Some(0) | None => Err(format!("channel.{name} must be a positive number of seconds")),
        Some(secs) => Ok(tk::TkDuration::from_secs(secs)),
    }
}

/// A rate.
fn rate_field(
    name: &str,
    user: Option<u32>,
    base: Option<u32>,
) -> Result<u32, String> {
    match user.or(base) {
        Some(0) | None => Err(format!("channel.{name} must be greater than zero")),
        Some(rate) => Ok(rate),
    }
}

impl Config {
    /// Build the runtime config from the file layer plus the arguments.
    pub(crate) fn from_toml(
        user: ConfigToml,
        id: String,
        namespace: String,
        work_dir: PathBuf,
        deny: DenySet,
        test: bool,
        remotes_file: PathBuf,
    ) -> Result<Self, String> {
        let base: ConfigToml = toml::from_str(DEFAULTS_CONFIG)
            .map_err(|e| format!("the embedded defaults do not parse: {e}"))?;
        let channel = merged_channel(user.channel, base.channel)?;
        let supervisor = merged_supervisor(user.supervisor, base.supervisor)?;
        let identity = resolved_identity(user.identity)?;
        Ok(Self {
            id,
            namespace,
            work_dir,
            deny,
            test,
            channel,
            supervisor,
            identity,
            remotes_file,
        })
    }

    /// Read a user file; an absent or malformed `--config` is an error.
    pub(crate) fn read_toml(path: &std::path::Path) -> Result<ConfigToml, String> {
        let text = fs::read_to_string(path)
            .map_err(|e| format!("read config {}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("parse config {}: {e}", path.display()))
    }

    /// Read the agent's `.grammar/config.toml` under `dir`; an absent one is none.
    pub(crate) fn read_agent_toml(dir: &std::path::Path) -> Result<ConfigToml, String> {
        let path = dir.join(AGENT_CONFIG_FILE);
        if !path.exists() {
            return Ok(ConfigToml::default());
        }
        Self::read_toml(&path)
    }
}

impl Default for Config {
    fn default() -> Self {
        let id = default_id();
        let work_dir = default_work_dir(&id);
        Self::from_toml(
            ConfigToml::default(),
            id,
            DEFAULT_NAMESPACE.to_string(),
            work_dir,
            DenySet::default(),
            false,
            Path::new(AGENT_CONFIG_FILE).with_file_name(REMOTES_FILE),
        )
        .expect("embedded defaults parse")
    }
}

/// The invoking user's name, for the zero-config human case.
pub(crate) fn default_id() -> String {
    std::env::var("USER").unwrap_or_else(|_| "default".to_string())
}

pub(crate) fn default_work_dir(id: &str) -> PathBuf {
    base_dirs().home_dir().join("proj").join("equip").join(id)
}

/// Expand `~` and `$VAR` references so config files stay portable.
pub(crate) fn expand_path(raw: &Path) -> Result<PathBuf, String> {
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
        match var_default(var) {
            Some(default) => Ok(Some(default.to_string_lossy().into_owned())),
            None => Err("environment variable unset or empty".to_string()),
        }
    })
    .map(|expanded| PathBuf::from(expanded.as_ref()))
    .map_err(|e| format!("expand {}: ${}: {}", raw.display(), e.var_name, e.cause))
}

/// `$HOME`, else Windows' `USERPROFILE`, read per call: the tilde's source.
fn home_dir() -> Option<String> {
    std::env::var("HOME")
        .ok()
        .or_else(|| cfg!(windows).then(|| std::env::var("USERPROFILE").ok()).flatten())
}

/// An unset variable's stand-in: a UENV home's standard variable, else a default.
pub(crate) fn var_default(var: &str) -> Option<PathBuf> {
    match uenv_standard(var) {
        Some(standard) => env_dir(standard).or_else(|| spec_default(standard)),
        None => spec_default(var),
    }
}

/// Each home variable's stand-in, as data a system's expansion reads.
pub(crate) fn expand_defaults() -> Vec<ExpandDefault> {
    home_vars()
        .filter_map(|var| {
            var_default(var).map(|value| ExpandDefault {
                var: var.to_string(),
                value: value.to_string_lossy().into_owned(),
            })
        })
        .collect()
}

/// The default for an unset standard home: the XDG spec's, and `TMPDIR`'s own.
pub(crate) fn spec_default(var: &str) -> Option<PathBuf> {
    let home_relative = match var {
        "TMPDIR" => return Some(tmp_default()),
        "XDG_CACHE_HOME" => ".cache",
        "XDG_CONFIG_HOME" => ".config",
        "XDG_DATA_HOME" => ".local/share",
        "XDG_STATE_HOME" => ".local/state",
        _ => return None,
    };
    home_dir().map(|home| PathBuf::from(home).join(home_relative))
}

/// A tool `--deny` withholds, or one interface command by its words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeniableTool {
    Execute,
    Rerun,
    Call,
    Interface,
    Command(&'static str),
}

impl DeniableTool {
    pub(crate) fn from_name(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "execute" => Self::Execute,
            "rerun" => Self::Rerun,
            "call" => Self::Call,
            "interface" => Self::Interface,
            words => Self::Command(command_by_words(words)?.words()),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DenySet {
    denied: Vec<DeniableTool>,
}

impl DenySet {
    pub(crate) fn new(denied: Vec<DeniableTool>) -> Self {
        Self { denied }
    }

    pub(crate) fn denies(
        &self,
        tool: DeniableTool,
    ) -> bool {
        self.denied.contains(&tool)
    }

    /// Whether the interface command its words name is withheld.
    pub(crate) fn denies_command(
        &self,
        words: &str,
    ) -> bool {
        self.denied
            .iter()
            .any(|tool| matches!(tool, DeniableTool::Command(denied) if *denied == words))
    }
}
