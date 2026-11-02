//! grammar-ssh's config: its namespace, homes, workdir, and supervisor lines.
use crate::*;

/// The agent's config file under a workdir, whose `[supervisor]` grammar-ssh reads.
pub const CONFIG_FILE: &str = ".grammar/config.toml";

/// The namespace grammar-ssh serves when none is named, the MCP's default.
pub(crate) const DEFAULT_NAMESPACE: &str = "default";

/// The MCP's embedded base, whose `[supervisor]` lines are grammar-ssh's defaults.
const BASE_CONFIG: &str = include_str!("../../../include/config/grammar_mcp.toml");

/// The components under a home where the MCP's namespaces sit, and grammar-ssh's.
const NAMESPACES: [&str; 2] = ["grammar", "mcp"];

/// The supervisor's lines as a config file sets them, each optional.
#[derive(Debug, Clone, Copy, Default, PartialEq, ser::Deserialize)]
#[serde(deny_unknown_fields)]
struct SupervisorLines {
    cpu_warn_fraction: Option<f64>,
    ram_warn_fraction: Option<f64>,
    vram_warn_headroom_mib: Option<u64>,
    disk_warn_fraction: Option<f64>,
}

/// A config file as grammar-ssh reads it: its `[supervisor]` table alone.
#[derive(Debug, Default, ser::Deserialize)]
struct ConfigFile {
    supervisor: Option<SupervisorLines>,
}

/// An id or a namespace, `[A-Za-z0-9][A-Za-z0-9._-]{0,63}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name(String);

impl Name {
    /// `text` as the `field` it names, refused where it isn't a name.
    pub fn parse(
        field: &'static str,
        text: &str,
    ) -> GrammarSshResult<Self> {
        if valid_name(text) {
            Ok(Self(text.to_string()))
        } else {
            Err(GrammarSshError::NameInvalid {
                field,
                name: text.to_string(),
            })
        }
    }

    /// The name's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// grammar-ssh's config: its namespace, homes, workdir, and supervisor lines.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// The effective Unix UID grammar-ssh runs as.
    pub uid: u32,
    /// The agent identity owning the namespace.
    pub id: String,
    /// The namespace within the id.
    pub namespace: String,
    /// The homes the namespace's directories sit under.
    pub homes: Homes,
    /// The working directory, a definition's `$env.EQUIP_WORK_DIR`.
    pub workdir: PathBuf,
    /// The supervisor's lines in force.
    pub supervisor: SupervisorConfig,
}

impl Config {
    /// `id` and `namespace`'s config for `uid` in `workdir`, its homes from `env`.
    pub fn load(
        uid: u32,
        id: &str,
        namespace: &str,
        workdir: &Path,
        env: &BTreeMap<String, String>,
    ) -> GrammarSshResult<Self> {
        let id = Name::parse("id", id)?;
        let namespace = Name::parse("namespace", namespace)?;
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
        let homes = Homes::resolve(env)?;
        let supervisor = read_supervisor(&workdir.join(CONFIG_FILE))?;
        Ok(Self {
            uid,
            id: id.as_str().to_string(),
            namespace: namespace.as_str().to_string(),
            homes,
            workdir: workdir.to_path_buf(),
            supervisor,
        })
    }

    /// The namespace's data home, the MCP's own.
    pub fn data_home(&self) -> PathBuf {
        self.namespace_in(&self.homes.data)
    }

    /// The namespace's cache home, the MCP's own.
    pub fn cache_home(&self) -> PathBuf {
        self.namespace_in(&self.homes.cache)
    }

    /// The namespace lock, the MCP's own in the data home.
    pub fn lock_file(&self) -> PathBuf {
        self.data_home().join(LOCK_FILE)
    }

    /// The namespace's directory under `home`.
    fn namespace_in(
        &self,
        home: &Path,
    ) -> PathBuf {
        NAMESPACES
            .iter()
            .fold(home.to_path_buf(), |path, part| path.join(part))
            .join(&self.id)
            .join(&self.namespace)
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
