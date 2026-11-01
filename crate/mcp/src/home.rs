use crate::*;

/// Each UENV home beside the standard variable it stands in front of.
const UENV_HOMES: [(&str, &str); 6] = [
    ("UENV_USR_CACHE", "XDG_CACHE_HOME"),
    ("UENV_USR_CONFIG", "XDG_CONFIG_HOME"),
    ("UENV_USR_DATA", "XDG_DATA_HOME"),
    ("UENV_USR_STATE", "XDG_STATE_HOME"),
    ("UENV_USR_TMP", "TMPDIR"),
    ("UENV_USR_TRANSIENT", "XDG_RUNTIME_DIR"),
];

/// The temporary directory where no variable names one, `/tmp` on Unix.
pub(crate) fn tmp_default() -> PathBuf {
    if cfg!(unix) {
        PathBuf::from("/tmp")
    } else {
        std::env::temp_dir()
    }
}

/// The user's base dirs, `None` where no home directory can be found.
pub(crate) static BASE_DIRS: LazyLock<Option<dirs::BaseDirs>> = LazyLock::new(dirs::BaseDirs::new);

/// Startup's check that a home directory can be found.
pub(crate) fn check_base_dirs() -> Result<(), String> {
    match BASE_DIRS.as_ref() {
        Some(_) => Ok(()),
        None => Err("grammar: no home directory can be found; set HOME".to_string()),
    }
}

/// The base dirs, which startup's check has found.
pub(crate) fn base_dirs() -> &'static dirs::BaseDirs {
    BASE_DIRS.as_ref().expect("base dirs checked at startup")
}

/// The standard variable a UENV home stands in front of.
pub(crate) fn uenv_standard(uenv: &str) -> Option<&'static str> {
    UENV_HOMES.iter().find(|(name, _)| *name == uenv).map(|(_, standard)| *standard)
}

/// The UENV home that stands in front of a standard variable.
pub(crate) fn uenv_partner(standard: &str) -> Option<&'static str> {
    UENV_HOMES.iter().find(|(_, name)| *name == standard).map(|(uenv, _)| *uenv)
}

/// Every home variable: each UENV home, then the standard behind it.
pub(crate) fn home_vars() -> impl Iterator<Item = &'static str> {
    UENV_HOMES.iter().flat_map(|(uenv, standard)| [*uenv, *standard])
}

/// The absolute path a variable holds; an empty or a relative one is ignored.
pub(crate) fn env_dir(name: &str) -> Option<PathBuf> {
    let value = std::env::var_os(name)?;
    let dir = PathBuf::from(&value);
    if dir.is_absolute() {
        return Some(dir);
    }
    if !value.is_empty() {
        eprintln!("grammar: {name} is ignored, not an absolute path: {}", dir.display());
    }
    None
}

/// A home from its UENV variable, else from the standard one behind it.
pub(crate) fn home_var(uenv: &str) -> Option<PathBuf> {
    env_dir(uenv).or_else(|| uenv_standard(uenv).and_then(env_dir))
}

/// The cache home's root, `UENV_USR_CACHE` or else XDG's, read once.
static CACHE_ROOT: LazyLock<PathBuf> = LazyLock::new(|| {
    env_dir("UENV_USR_CACHE").unwrap_or_else(|| base_dirs().cache_dir().to_path_buf())
});

/// The data home's root, `UENV_USR_DATA` or else XDG's, read once.
static DATA_ROOT: LazyLock<PathBuf> = LazyLock::new(|| {
    env_dir("UENV_USR_DATA").unwrap_or_else(|| base_dirs().data_dir().to_path_buf())
});

impl Config {
    /// The namespace's cache home.
    pub(crate) fn cache_home(&self) -> PathBuf {
        CACHE_ROOT
            .join(consts::GRAMMAR)
            .join(consts::MCP)
            .join(&self.id)
            .join(&self.namespace)
    }

    /// The namespace's data home.
    pub(crate) fn data_home(&self) -> PathBuf {
        DATA_ROOT
            .join(consts::GRAMMAR)
            .join(consts::MCP)
            .join(&self.id)
            .join(&self.namespace)
    }
}
