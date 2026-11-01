use crate::*;

/// The stack every runtime worker gets, the size of an eval thread's.
const WORKER_STACK_SIZE: usize = 64 * 1024 * 1024;

const _: () = assert!(WORKER_STACK_SIZE >= EVAL_STACK_SIZE);

/// What the host's record in its namespace lock names its holder.
const LOCK_HOLDER: &str = "grammar-mcp";

/// Set a home from its flag, under its UENV name too, so the flag wins.
fn set_cli_env_path(k: &'static str, v: &Option<PathBuf>) -> Result<(), String> {
    let Some(v) = v else { return Ok(()) };
    let v = expand_path(v)?;
    unsafe { std::env::set_var(k, &v) }
    if let Some(uenv) = uenv_partner(k) {
        unsafe { std::env::set_var(uenv, &v) }
    }
    Ok(())
}

pub(crate) fn set_cli_env(cli: &Cli) -> Result<(), String> {
    let Some(env) = &cli.env else { return Ok(()) };

    set_cli_env_path("XDG_CACHE_HOME", &env.xdg_cache_home)?;
    set_cli_env_path("XDG_CONFIG_HOME", &env.xdg_config_home)?;
    set_cli_env_path("XDG_DATA_HOME", &env.xdg_data_home)?;
    set_cli_env_path("XDG_STATE_HOME", &env.xdg_state_home)?;

    if let Some(vars) = &env.vars {
        for (var,val) in vars {
            unsafe { std::env::set_var(var, val); }
        }
    }
    Ok(())
}

pub fn run_main() -> process::ExitCode {
    let cli = Cli::parse();
    run_with(cli)
}

pub fn run_with(cli: Cli) -> process::ExitCode {
    if let Err(reason) = set_cli_env(&cli) {
        eprintln!("{reason}");
        return process::ExitCode::from(2);
    }
    run(cli)
}

fn run(cli: Cli) -> process::ExitCode {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(WORKER_STACK_SIZE)
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("grammar: the tokio runtime: {e}");
            return process::ExitCode::FAILURE;
        }
    };
    runtime.block_on(async {
        match tk::spawn(serve_or_oneshot(cli)).await {
            Ok(code) => code,
            Err(e) => {
                eprintln!("grammar: the host task: {e}");
                process::ExitCode::FAILURE
            }
        }
    })
}

async fn serve_or_oneshot(cli: Cli) -> process::ExitCode {
    if let Err(reason) = check_base_dirs() {
        eprintln!("{reason}");
        return process::ExitCode::from(2);
    }
    let (config, command) = match resolve_config(cli) {
        Ok(resolved) => resolved,
        Err(reason) => {
            eprintln!("{reason}");
            return process::ExitCode::from(2);
        }
    };
    let config = Arc::new(config);
    let code = match command {
        None => serve(config).await,
        Some(CliCmd::Cli { tool }) => run_cli(config, tool).await,
    };
    remove_scratch_dirs();
    code
}

/// The server: MCP over stdio on the in-process engine until the agent leaves.
async fn serve(config: Arc<Config>) -> process::ExitCode {
    let lock = match lock_namespace(&config) {
        Ok(lock) => lock,
        Err(reason) => {
            eprintln!("grammar: {reason}");
            return process::ExitCode::FAILURE;
        }
    };
    if let Err(reason) = init_engine(&config) {
        eprintln!("{reason}");
        return process::ExitCode::from(2);
    }
    match run_server(config, lock).await {
        Ok(()) => process::ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("grammar: {reason}");
            process::ExitCode::FAILURE
        }
    }
}

/// The local CLI: one tool call on the in-process engine, its envelope printed.
async fn run_cli(
    config: Arc<Config>,
    tool: CliTool,
) -> process::ExitCode {
    let lock = match lock_namespace(&config) {
        Ok(lock) => lock,
        Err(reason) => {
            eprintln!("grammar: {reason}");
            return process::ExitCode::FAILURE;
        }
    };
    if let Err(reason) = init_engine(&config) {
        eprintln!("{reason}");
        return process::ExitCode::from(2);
    }
    run_oneshot(config, tool, lock).await
}

/// The namespace's lock taken and this process recorded in it.
pub(crate) fn lock_namespace(config: &Config) -> Result<NamespaceLock, String> {
    let mut lock = NamespaceLock::acquire(&config.data_home().join(LOCK_FILE)).map_err(|e| e.to_string())?;
    let (_, mcp_nom) = process_identity();
    let record = LockRecord::current(LOCK_HOLDER, &config.id, &config.namespace, mcp_nom.as_str())
        .map_err(|e| format!("the namespace lock's record: {e}"))?;
    lock.record(&record).map_err(|e| e.to_string())?;
    Ok(lock)
}

/// `--test` supplies a default namespace, never an override.
fn resolve_namespace(
    namespace: Option<String>,
    test: bool,
) -> String {
    namespace.unwrap_or_else(|| {
        if test {
            TEST_NAMESPACE.to_string()
        } else {
            DEFAULT_NAMESPACE.to_string()
        }
    })
}

/// Expand a work-dir argument the same way a config path is expanded.
fn resolve_work_dir(
    raw: Option<&Path>,
    id: &str,
) -> Result<PathBuf, String> {
    match raw {
        Some(s) => expand_path(s),
        None => Ok(default_work_dir(id)),
    }
}

pub(crate) fn parse_deniable(s: &str) -> Result<DeniableTool, String> {
    DeniableTool::from_name(s).ok_or_else(|| {
        let commands: Vec<&str> = COMMANDS.iter().map(|spec| spec.words()).collect();
        format!(
            "unknown tool `{s}`; deniable: execute, rerun, call, interface, and the interface \
             commands by their words: {}",
            commands.join(", "),
        )
    })
}

/// The argument-owned values plus whatever the file contributes.
fn resolve_config(cli: Cli) -> Result<(Config, Option<CliCmd>), String> {
    let (file, config_file) = match &cli.config {
        Some(raw) => {
            let path = expand_path(raw)?;
            (Config::read_toml(&path)?, path)
        }
        None => {
            let start_dir = std::env::current_dir()
                .map_err(|e| format!("cannot read the start directory: {e}"))?;
            (Config::read_agent_toml(&start_dir)?, start_dir.join(AGENT_CONFIG_FILE))
        }
    };
    let work_dir = resolve_work_dir(cli.workdir.as_deref(), &cli.id)?;
    let namespace = resolve_namespace(cli.namespace, cli.test);
    let config = Config::from_toml(
        file,
        cli.id,
        namespace,
        work_dir,
        DenySet::new(cli.deny),
        cli.test,
        config_file.with_file_name(REMOTES_FILE),
    )?;
    Ok((config, cli.command))
}
