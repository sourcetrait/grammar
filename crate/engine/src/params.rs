use crate::*;

/// What the engine takes from its host when it starts.
#[derive(Debug, Clone)]
pub struct EngineParams {
    /// The host's id, a definition's `$env.EQUIP_ID`.
    pub id: String,
    /// The host's namespace, a definition's `$env.EQUIP_NAMESPACE`.
    pub namespace: String,
    /// The host's work dir, a definition's `$env.EQUIP_WORK_DIR`.
    pub work_dir: PathBuf,
    /// The host's bulk scratch directory, a definition's `$env.EQUIP_TMP`.
    pub tmp_dir: PathBuf,
    /// The host's transient scratch directory, a definition's `$env.EQUIP_TRANSIENT`.
    pub transient_dir: PathBuf,
    /// The host process's nom, a definition's `$env.EQUIP_MCP_NOM`.
    pub mcp_nom: String,
    /// The namespace's data home: the lib dirs and the plugin registry.
    pub data_home: PathBuf,
    /// The namespace's cache home: the call logs and the cached definitions.
    pub cache_home: PathBuf,
}

/// The parameters `init` set, once.
static PARAMS: OnceLock<EngineParams> = OnceLock::new();

/// The engine's parameters, `None` before `init`.
pub(crate) fn params() -> Option<&'static EngineParams> {
    PARAMS.get()
}

/// The parameters an engine reads, which `init` sets before any engine starts.
pub(crate) fn started() -> &'static EngineParams {
    params().expect("the engine's params set by init")
}

/// The namespace's data home.
pub(crate) fn data_home() -> PathBuf {
    started().data_home.clone()
}

/// The namespace's cache home.
pub(crate) fn cache_home() -> PathBuf {
    started().cache_home.clone()
}

/// Starts the engine on its parameters, once.
pub fn init(params: EngineParams) -> EngineResult<()> {
    PARAMS.set(params).map_err(|_| EngineError::Init {
        reason: "grammar: the engine is already started".to_string(),
    })?;
    init_config_dirs().map_err(|reason| EngineError::Init { reason })
}
