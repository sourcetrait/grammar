//! grammar-ssh's entry: its controller started, its one session served, its end.
use crate::*;

/// The stack every runtime worker gets, the size of an eval thread's.
const WORKER_STACK_SIZE: usize = 64 * 1024 * 1024;

const _: () = assert!(WORKER_STACK_SIZE >= EVAL_STACK_SIZE);

/// How long the end waits for the runtime's tasks to finish.
const SHUTDOWN_GRACE: std::time::Duration = std::time::Duration::from_secs(2);

/// Parses grammar-ssh's command line, serves its session, and exits by its result.
pub fn run_main() -> process::ExitCode {
    let cli = Cli::parse();
    let ended = wire()
        .map_err(|e| GrammarSshError::Session {
            reason: format!("the wire's descriptors: {e}"),
        })
        .and_then(|(reader, writer)| {
            let runtime = tk::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_stack_size(WORKER_STACK_SIZE)
                .build()
                .map_err(|e| GrammarSshError::TasksUnavailable { reason: e.to_string() })?;
            let ended = runtime.block_on(run(cli, reader, writer));
            runtime.shutdown_timeout(SHUTDOWN_GRACE);
            ended
        });
    match ended {
        Ok(end) => {
            eprintln!("grammar: {end}, stopping");
            process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("grammar: {e}");
            process::ExitCode::from(e.exit_code())
        }
    }
}

/// The wire: stdin, and stdout's descriptor close-on-exec, fd 1 given stderr.
#[cfg(unix)]
fn wire() -> io::Result<(tk::Stdin, tk::File)> {
    use std::os::fd::AsFd;
    let out = io::stdout().as_fd().try_clone_to_owned()?;
    nix::unistd::dup2_stdout(io::stderr())?;
    Ok((tk::stdin(), tk::File::from_std(fs::File::from(out))))
}

/// The wire: stdin, and stdout's handle uninheritable, stdout given stderr.
#[cfg(windows)]
fn wire() -> io::Result<(tk::Stdin, tk::File)> {
    use std::os::windows::io::{AsHandle, AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::System::Console::{STD_OUTPUT_HANDLE, SetStdHandle};
    let stdout = io::stdout();
    let out = stdout.as_handle().try_clone_to_owned()?;
    let old = stdout.as_raw_handle();
    let stderr = io::stderr().as_raw_handle();
    if unsafe { SetStdHandle(STD_OUTPUT_HANDLE, stderr) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if old != stderr {
        drop(unsafe { OwnedHandle::from_raw_handle(old) });
    }
    Ok((tk::stdin(), tk::File::from_std(fs::File::from(out))))
}

/// The controller started and its session served, or its start's failure refused.
async fn run(
    cli: Cli,
    reader: tk::Stdin,
    writer: tk::File,
) -> GrammarSshResult<SessionEnd> {
    let controller = match start(&cli).await {
        Ok(controller) => controller,
        Err(e) => {
            let refusal = TransportError::Unavailable {
                reason: format!("{}: {e}", e.kind().name()),
            };
            refuse(reader, writer, refusal).await;
            return Err(e);
        }
    };
    let config = controller.config();
    eprintln!(
        "grammar: serving {}/{} as {}, pid {}",
        config.id,
        config.namespace,
        controller.nom(),
        process::id(),
    );
    let end = controller.serve(reader, writer).await;
    let shut = controller.shutdown().await;
    let end = end?;
    shut?;
    Ok(end)
}

/// The config, the workdir entered, the lock, the scratch, and the engine's start.
async fn start(cli: &Cli) -> GrammarSshResult<GrammarSshController> {
    let env: BTreeMap<String, String> = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    let workdir = start_dir(cli.workdir.as_deref())?;
    let config = Config::load(current_uid(), &cli.id, &cli.namespace, &workdir, &env)?;
    for line in &config.homes.diagnostics {
        eprintln!("grammar: {line}");
    }
    enter(&config.workdir)?;
    let lock = NamespaceLock::acquire(&config.lock_file())?;
    let scratch = ScratchDirs::create(&config.homes)?;
    for line in scratch.diagnostics() {
        eprintln!("grammar: {line}");
    }
    if let Err(e) = init_engine(&config, &scratch).and_then(|()| ensure_substrate()) {
        let _ = scratch.remove();
        return Err(e);
    }
    GrammarSshController::start(Arc::new(config), scratch, lock).await
}

/// The workdir named, or the directory the session started in when none is.
fn start_dir(workdir: Option<&Path>) -> GrammarSshResult<PathBuf> {
    match workdir {
        Some(workdir) => Ok(workdir.to_path_buf()),
        None => std::env::current_dir().map_err(|e| GrammarSshError::WorkdirInvalid {
            path: PathBuf::from("."),
            reason: format!("the session's start directory: {e}"),
        }),
    }
}

/// The process moved into `workdir`, before the engine seeds its `PWD` from it.
fn enter(workdir: &Path) -> GrammarSshResult<()> {
    std::env::set_current_dir(workdir).map_err(|e| GrammarSshError::WorkdirInvalid {
        path: workdir.to_path_buf(),
        reason: format!("entering it: {e}"),
    })
}
