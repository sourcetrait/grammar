use crate::*;

/// The nushell version the engine embeds, read from the lockfile at build.
pub const NU_VERSION: &str = env!("NU_VERSION");

/// The command set every engine shares: nushell's language, shell, and extras.
fn base_context() -> nu::EngineState {
    let mut engine_state = nu::add_shell_command_context(nu::create_default_context());
    engine_state = nu::add_extra_command_context(engine_state);
    engine_state.is_interactive = false;
    engine_state.is_mcp = true;
    engine_state
}

/// An engine for `mode`, started in the order nushell's own startup runs.
pub(crate) fn startup(mode: Mode) -> nu::EngineState {
    let mut engine_state = base_context();
    if matches!(mode, Mode::Stateful) {
        engine_state = nu::add_plugin_command_context(engine_state);
    }
    shadow_host_fatal_decls(&mut engine_state);
    add_print(&mut engine_state);
    engine_state.config_dirs = config_dirs().clone();
    seed_plugin_dirs(&mut engine_state);
    seed_config(&mut engine_state);
    seed_env(&mut engine_state);
    convert_env(&mut engine_state);
    seed_lib_dirs(&mut engine_state);
    seed_version(&mut engine_state);
    load_std(&mut engine_state);
    load_plugin_decls(&mut engine_state);
    engine_state.generate_nu_constant();
    #[cfg(feature = "test-hooks")]
    crate::test_hooks::register_test_hooks(&mut engine_state);
    engine_state
}

/// The host's `print`, which writes to the running call's stderr log.
fn add_print(engine_state: &mut nu::EngineState) {
    let mut working_set = nu::StateWorkingSet::new(engine_state);
    working_set.add_decl(Box::new(HostPrint));
    let delta = working_set.render();
    let _ = engine_state.merge_delta(delta);
}

/// `NU_PLUGIN_DIRS` empty and `$NU_PLUGIN_DIRS` as nushell's startup sets them.
fn seed_plugin_dirs(engine_state: &mut nu::EngineState) {
    let span = nu::Span::unknown();
    engine_state.add_env_var("NU_PLUGIN_DIRS".to_string(), nu::Value::list(Vec::new(), span));
    let mut dirs = vec![engine_state.config_dirs.config_home.join("plugins")];
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        dirs.push(exe_dir);
    }
    set_dirs_const(engine_state, PLUGIN_DIRS_CONST, &dirs);
}

/// `$env.config` and `ENV_CONVERSIONS` at nushell's defaults.
fn seed_config(engine_state: &mut nu::EngineState) {
    let span = nu::Span::unknown();
    engine_state.add_env_var("config".to_string(), nu::Config::default().into_value(span));
    engine_state.add_env_var(
        "ENV_CONVERSIONS".to_string(),
        nu::Value::record(nu::Record::new(), span),
    );
}

/// Seed the process env into `$env`, plus the ambient EQUIP six.
fn seed_env(engine_state: &mut nu::EngineState) {
    if let Ok(cwd) = std::env::current_dir() {
        engine_state.add_env_var(
            "PWD".to_string(),
            nu::Value::string(cwd.to_string_lossy().into_owned(), nu::Span::unknown()),
        );
    }
    for (key, val) in std::env::vars() {
        if key == "PWD" || key == "NU_LIB_DIRS" {
            continue;
        }
        engine_state.add_env_var(key, nu::Value::string(val, nu::Span::unknown()));
    }
    let params = started();
    let span = nu::Span::unknown();
    engine_state.add_env_var("EQUIP_ID".to_string(), nu::Value::string(params.id.clone(), span));
    engine_state.add_env_var(
        "EQUIP_NAMESPACE".to_string(),
        nu::Value::string(params.namespace.clone(), span),
    );
    engine_state.add_env_var(
        "EQUIP_WORK_DIR".to_string(),
        nu::Value::string(params.work_dir.to_string_lossy().into_owned(), span),
    );
    engine_state.add_env_var(
        "EQUIP_TMP".to_string(),
        nu::Value::string(params.tmp_dir.to_string_lossy().into_owned(), span),
    );
    engine_state.add_env_var(
        "EQUIP_TRANSIENT".to_string(),
        nu::Value::string(params.transient_dir.to_string_lossy().into_owned(), span),
    );
    engine_state.add_env_var(
        "EQUIP_MCP_NOM".to_string(),
        nu::Value::string(params.mcp_nom.clone(), span),
    );
}

/// The env's strings converted as nushell converts them, `PATH` into a list.
fn convert_env(engine_state: &mut nu::EngineState) {
    let mut stack = nu::Stack::new();
    if let Err(e) = nu::convert_env_values(engine_state, &mut stack) {
        eprintln!("grammar: the env's conversion failed: {e}");
    }
    merge_env_no_chdir(engine_state, &mut stack);
}

/// The namespace dir as the lib dirs, both `$NU_LIB_DIRS` and `$env.NU_LIB_DIRS`.
fn seed_lib_dirs(engine_state: &mut nu::EngineState) {
    let dirs = [data_home()];
    engine_state.add_env_var("NU_LIB_DIRS".to_string(), dirs_value(&dirs));
    set_dirs_const(engine_state, LIB_DIRS_CONST, &dirs);
}

/// `NU_VERSION` the engine's own version, never the parent shell's.
fn seed_version(engine_state: &mut nu::EngineState) {
    engine_state.add_env_var(
        "NU_VERSION".to_string(),
        nu::Value::string(NU_VERSION, nu::Span::unknown()),
    );
}

/// nushell's standard library, loaded as nushell's startup loads it.
fn load_std(engine_state: &mut nu::EngineState) {
    if let Err(e) = nu::load_standard_library(engine_state) {
        eprintln!("grammar: the standard library failed to load: {e:?}");
    }
}
