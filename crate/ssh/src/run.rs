//! grammar-ssh's entry: its command line parsed and its one session served.
use crate::*;

/// Parses grammar-ssh's command line, serves its session, and exits by its result.
pub fn run_main() -> process::ExitCode {
    let GrammarSshCmd::Serve { id, namespace, workdir } = GrammarSshCli::parse().command;
    match serve(&id, &namespace, workdir.as_deref()) {
        Ok(end) => {
            eprintln!("grammar-ssh: {end}, stopping");
            process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("grammar-ssh: {e}");
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

/// `id` and `namespace`'s session, in `workdir` or its start, to its end.
fn serve(
    id: &str,
    namespace: &str,
    workdir: Option<&Path>,
) -> GrammarSshResult<SessionEnd> {
    let (reader, writer) = wire().map_err(|e| GrammarSshError::Session {
        reason: format!("the wire's descriptors: {e}"),
    })?;
    let env: BTreeMap<String, String> = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    let started = start_dir(workdir).and_then(|workdir| {
        InstanceConfig::load(current_uid(), id, namespace, &workdir, &env)
    });
    let started = started.and_then(|config| {
        for line in &config.homes.diagnostics {
            eprintln!("grammar-ssh: {line}");
        }
        enter(&config.workdir)?;
        Runtime::start(config)
    });
    let runtime = match started {
        Ok(runtime) => runtime,
        Err(e) => {
            refuse_with(reader, writer, &e);
            return Err(e);
        }
    };
    for line in runtime.diagnostics() {
        eprintln!("grammar-ssh: {line}");
    }
    eprintln!(
        "grammar-ssh: serving {}/{} as {}, pid {}",
        runtime.identity().id,
        runtime.identity().namespace,
        runtime.nom(),
        process::id(),
    );
    let end = runtime.serve(reader, writer);
    let shut = runtime.shutdown();
    let end = end?;
    shut?;
    Ok(end)
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

/// A start's failure told the MCP: its hello read and refused with the reason.
fn refuse_with(
    reader: tk::Stdin,
    writer: tk::File,
    e: &GrammarSshError,
) {
    let refusal = TransportError::Unavailable {
        reason: format!("{}: {e}", e.kind().name()),
    };
    match tk::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(tasks) => tasks.block_on(refuse(reader, writer, refusal)),
        Err(io) => eprintln!("grammar-ssh: the refusal's runtime: {io}"),
    }
}
