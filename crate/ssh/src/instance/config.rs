//! A worker's instance: its identity, homes, paths, workdir, and supervisor lines.
use crate::*;

/// The agent's config file under a workdir, whose `[supervisor]` a worker reads.
pub const CONFIG_FILE: &str = ".grammar/config.toml";

/// The MCP's embedded base, whose `[supervisor]` lines are a worker's defaults.
const BASE_CONFIG: &str = include_str!("../../../../include/config/grammar_mcp.toml");

/// The supervisor's lines as a config file sets them, each optional.
#[derive(Debug, Clone, Copy, Default, PartialEq, ser::Deserialize)]
#[serde(deny_unknown_fields)]
struct SupervisorLines {
    cpu_warn_fraction: Option<f64>,
    ram_warn_fraction: Option<f64>,
    vram_warn_headroom_mib: Option<u64>,
    disk_warn_fraction: Option<f64>,
}

/// A config file as a worker reads it: its `[supervisor]` table alone.
#[derive(Debug, Default, ser::Deserialize)]
struct ConfigFile {
    supervisor: Option<SupervisorLines>,
}

/// Where a worker's files are, each from its identity and its homes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstancePaths {
    /// The engine's data home, the MCP's own layout.
    pub data: PathBuf,
    /// The engine's cache home, the MCP's own layout.
    pub cache: PathBuf,
    /// The namespace lock, the MCP's own in the engine's data home.
    pub lock: PathBuf,
}

impl InstancePaths {
    /// The paths of `id` and `namespace`'s instance under `homes`.
    pub fn of(
        homes: &InstanceHomes,
        id: &InstanceName,
        namespace: &InstanceName,
    ) -> Self {
        let mcp = |home: &Path| home.join("grammar").join("mcp").join(id.as_str()).join(namespace.as_str());
        Self {
            data: mcp(&homes.data),
            cache: mcp(&homes.cache),
            lock: mcp(&homes.data).join(LOCK_FILE),
        }
    }
}

/// A worker's instance: its identity, homes, paths, workdir, and supervisor lines.
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceConfig {
    pub identity: RuntimeIdentity,
    pub homes: InstanceHomes,
    pub paths: InstancePaths,
    pub workdir: PathBuf,
    pub supervisor: SupervisorConfig,
}

impl InstanceConfig {
    /// `id` and `namespace`'s instance for `uid` in `workdir`, its homes from `env`.
    pub fn load(
        uid: u32,
        id: &str,
        namespace: &str,
        workdir: &Path,
        env: &BTreeMap<String, String>,
    ) -> GrammarSshResult<Self> {
        let id = InstanceName::parse("id", id)?;
        let namespace = InstanceName::parse("namespace", namespace)?;
        let invalid = |reason: &str| GrammarSshError::WorkdirInvalid {
            path: workdir.to_path_buf(),
            reason: reason.to_string(),
        };
        if !workdir.is_absolute() {
            return Err(invalid("it is not an absolute path"));
        }
        if !workdir.is_dir() {
            return Err(invalid("it is not a directory"));
        }
        let homes = InstanceHomes::resolve(env)?;
        let paths = InstancePaths::of(&homes, &id, &namespace);
        let supervisor = read_supervisor(&workdir.join(CONFIG_FILE))?;
        let identity = RuntimeIdentity {
            uid,
            id: id.as_str().to_string(),
            namespace: namespace.as_str().to_string(),
        };
        Ok(Self {
            identity,
            homes,
            paths,
            workdir: workdir.to_path_buf(),
            supervisor,
        })
    }
}

/// The supervisor lines in force: `file`'s, where it sets them, over the base's.
fn read_supervisor(file: &Path) -> GrammarSshResult<SupervisorConfig> {
    let invalid = |reason: String| GrammarSshError::ConfigInvalid {
        path: file.to_path_buf(),
        reason,
    };
    let base = toml::from_str::<ConfigFile>(BASE_CONFIG)
        .map_err(|e| invalid(format!("the embedded base: {e}")))?
        .supervisor
        .unwrap_or_default();
    let user = match fs::read_to_string(file) {
        Ok(text) => toml::from_str::<ConfigFile>(&text).map_err(|e| invalid(e.to_string()))?.supervisor.unwrap_or_default(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => SupervisorLines::default(),
        Err(e) => return Err(invalid(e.to_string())),
    };
    let fraction = |name: &str, user: Option<f64>, base: Option<f64>| {
        let value = user.or(base).ok_or_else(|| invalid(format!("the embedded base carries no supervisor.{name}")))?;
        check_fraction(name, value).map_err(invalid)
    };
    Ok(SupervisorConfig {
        cpu_warn_fraction: fraction("cpu_warn_fraction", user.cpu_warn_fraction, base.cpu_warn_fraction)?,
        ram_warn_fraction: fraction("ram_warn_fraction", user.ram_warn_fraction, base.ram_warn_fraction)?,
        vram_warn_headroom_mib: user
            .vram_warn_headroom_mib
            .or(base.vram_warn_headroom_mib)
            .ok_or_else(|| invalid("the embedded base carries no supervisor.vram_warn_headroom_mib".to_string()))?,
        disk_warn_fraction: fraction("disk_warn_fraction", user.disk_warn_fraction, base.disk_warn_fraction)?,
    })
}
