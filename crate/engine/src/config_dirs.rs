use crate::*;

/// nushell's config dirs for this host, resolved once at startup.
pub(crate) static CONFIG_DIRS: OnceLock<nu::NushellConfigDirs> = OnceLock::new();

/// nushell's config dirs for this host, as resolved at startup.
pub(crate) fn config_dirs() -> &'static nu::NushellConfigDirs {
    CONFIG_DIRS.get().expect("config dirs resolved at startup")
}

/// nushell's config dirs for `env`, with `registry` as the plugin file.
pub(crate) fn resolve_config_dirs(
    env: &impl nu::EnvAccess,
    registry: &Path,
) -> Result<nu::NushellConfigDirs, String> {
    let overrides = nu::CliOverrides {
        plugin_file: Some(registry.to_path_buf()),
        ..Default::default()
    };
    let (dirs, warnings) = nu::resolve_paths(env, &overrides)
        .map_err(|e| format!("grammar: nushell's config dirs: {e}"))?;
    for warning in warnings {
        eprintln!("grammar: {warning}");
    }
    Ok(dirs)
}

/// Resolve nushell's config dirs from the process env and hold them, once.
pub(crate) fn init_config_dirs() -> Result<(), String> {
    let registry = registry_path()
        .ok_or_else(|| "grammar: nushell's config dirs need the engine's params".to_string())?;
    let dirs = resolve_config_dirs(&nu::SystemEnv, &registry)?;
    let _ = CONFIG_DIRS.set(dirs);
    Ok(())
}
