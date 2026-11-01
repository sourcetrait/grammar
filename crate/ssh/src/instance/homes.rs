//! The homes a worker resolves at its start, from its environment given as data.
use crate::*;

/// The temporary home's default where neither of its variables names one.
const TMP_DEFAULT: &str = "/tmp";

/// One home: its UENV variable, the standard one behind it, and its default.
struct HomeSpec {
    name: &'static str,
    uenv: &'static str,
    standard: Option<&'static str>,
    default: Default,
}

/// Where a home falls back to when neither variable names it.
enum Default {
    /// A directory beneath `HOME`.
    UnderHome(&'static str),
    /// An absolute directory of its own.
    Absolute(&'static str),
    /// None: the home stays unresolved.
    Unset,
}

const TMP: HomeSpec = HomeSpec { name: "tmp", uenv: "UENV_USR_TMP", standard: Some("TMPDIR"), default: Default::Absolute(TMP_DEFAULT) };
const RUNTIME: HomeSpec = HomeSpec { name: "runtime", uenv: "UENV_USR_TRANSIENT", standard: Some("XDG_RUNTIME_DIR"), default: Default::Unset };

/// The data home: its UENV variable, else XDG's, as the MCP resolves it.
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
const DATA: HomeSpec = HomeSpec { name: "data", uenv: "UENV_USR_DATA", standard: Some("XDG_DATA_HOME"), default: Default::UnderHome(".local/share") };

/// The cache home: its UENV variable, else XDG's, as the MCP resolves it.
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
const CACHE: HomeSpec = HomeSpec { name: "cache", uenv: "UENV_USR_CACHE", standard: Some("XDG_CACHE_HOME"), default: Default::UnderHome(".cache") };

/// The data home: its UENV variable, else the platform's, as the MCP resolves it.
#[cfg(any(target_os = "macos", target_os = "ios"))]
const DATA: HomeSpec = HomeSpec { name: "data", uenv: "UENV_USR_DATA", standard: None, default: Default::UnderHome("Library/Application Support") };

/// The cache home: its UENV variable, else the platform's, as the MCP resolves it.
#[cfg(any(target_os = "macos", target_os = "ios"))]
const CACHE: HomeSpec = HomeSpec { name: "cache", uenv: "UENV_USR_CACHE", standard: None, default: Default::UnderHome("Library/Caches") };

/// A worker's homes, each an existing absolute directory, the runtime's optional.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceHomes {
    pub data: PathBuf,
    pub cache: PathBuf,
    pub tmp: PathBuf,
    /// The runtime home, none where no variable names one.
    pub runtime: Option<PathBuf>,
    /// Each variable ignored on the way, and why.
    pub diagnostics: Vec<String>,
}

impl InstanceHomes {
    /// Every home from `env`, each validated and none made.
    pub fn resolve(env: &BTreeMap<String, String>) -> GrammarSshResult<Self> {
        let mut diagnostics = Vec::new();
        let mut required = |spec: &HomeSpec| -> GrammarSshResult<PathBuf> {
            resolve_home(spec, env, &mut diagnostics)?.ok_or_else(|| GrammarSshError::HomeUnavailable {
                home: spec.name,
                reason: "no variable names it and it has no default".to_string(),
            })
        };
        let data = required(&DATA)?;
        let cache = required(&CACHE)?;
        let tmp = required(&TMP)?;
        let runtime = resolve_home(&RUNTIME, env, &mut diagnostics)?;
        Ok(Self {
            data,
            cache,
            tmp,
            runtime,
            diagnostics,
        })
    }
}

/// An absolute path a variable holds, its being empty or relative noted instead.
fn named(
    env: &BTreeMap<String, String>,
    var: &str,
    diagnostics: &mut Vec<String>,
) -> Option<PathBuf> {
    let value = env.get(var)?;
    if value.is_empty() {
        diagnostics.push(format!("{var} is ignored, empty"));
        return None;
    }
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Some(path)
    } else {
        diagnostics.push(format!("{var} is ignored, not an absolute path: {value}"));
        None
    }
}

/// One home, UENV first, then its standard variable, then its default, validated.
fn resolve_home(
    spec: &HomeSpec,
    env: &BTreeMap<String, String>,
    diagnostics: &mut Vec<String>,
) -> GrammarSshResult<Option<PathBuf>> {
    let chosen = match named(env, spec.uenv, diagnostics).or_else(|| spec.standard.and_then(|standard| named(env, standard, diagnostics))) {
        Some(path) => Some(path),
        None => match spec.default {
            Default::UnderHome(rel) => match named(env, "HOME", diagnostics) {
                Some(home) => Some(home.join(rel)),
                None => {
                    return Err(GrammarSshError::HomeUnavailable {
                        home: spec.name,
                        reason: "HOME names no absolute directory".to_string(),
                    });
                }
            },
            Default::Absolute(path) => Some(PathBuf::from(path)),
            Default::Unset => None,
        },
    };
    let Some(path) = chosen else {
        return Ok(None);
    };
    if !path.is_dir() {
        return Err(GrammarSshError::HomeUnavailable {
            home: spec.name,
            reason: format!("{} is not a directory", path.display()),
        });
    }
    Ok(Some(path))
}
