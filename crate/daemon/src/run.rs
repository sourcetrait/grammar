//! grammard's entry: its command line parsed and its one session served.
use crate::*;

/// Parses grammard's command line, serves its session, and exits by its result.
pub fn run_main() -> process::ExitCode {
    let GrammardCmd::Serve { id, namespace, workdir } = GrammardCli::parse().command;
    match serve(&id, &namespace, &workdir) {
        Ok(end) => {
            eprintln!("grammard: {end}, stopping");
            process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("grammard: {e}");
            process::ExitCode::from(e.exit_code())
        }
    }
}

/// The wire's two ends: stdin, and stdout's own descriptor, fd 1 then given stderr.
fn wire() -> io::Result<(tk::Stdin, tk::File)> {
    let out = nix::unistd::dup(io::stdout())?;
    nix::unistd::dup2_stdout(io::stderr())?;
    Ok((tk::stdin(), tk::File::from_std(fs::File::from(out))))
}

/// `id` and `namespace`'s session in `workdir`, served until it ends.
fn serve(
    id: &str,
    namespace: &str,
    workdir: &Path,
) -> DaemonResult<SessionEnd> {
    let (reader, writer) = wire().map_err(|e| DaemonError::Session {
        reason: format!("the wire's descriptors: {e}"),
    })?;
    let env: BTreeMap<String, String> = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    let started = InstanceConfig::load(current_uid(), id, namespace, workdir, &env).and_then(|config| {
        for line in &config.homes.diagnostics {
            eprintln!("grammard: {line}");
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
        eprintln!("grammard: {line}");
    }
    eprintln!(
        "grammard: serving {}/{} as {}, pid {}",
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

/// The process moved into `workdir`, before the engine seeds its `PWD` from it.
fn enter(workdir: &Path) -> DaemonResult<()> {
    std::env::set_current_dir(workdir).map_err(|e| DaemonError::WorkdirInvalid {
        path: workdir.to_path_buf(),
        reason: format!("entering it: {e}"),
    })
}

/// A start's failure told the MCP: its hello read and refused with the reason.
fn refuse_with(
    reader: tk::Stdin,
    writer: tk::File,
    e: &DaemonError,
) {
    let refusal = TransportError::Unavailable {
        reason: format!("{}: {e}", e.kind().name()),
    };
    match tk::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(tasks) => tasks.block_on(refuse(reader, writer, refusal)),
        Err(io) => eprintln!("grammard: the refusal's runtime: {io}"),
    }
}
