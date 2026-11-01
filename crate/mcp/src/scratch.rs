//! The host's own scratch directories: one for bulk, one transient, a process.
use crate::*;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

/// A scratch directory's name prefix, which the sweep matches.
const SCRATCH_PREFIX: &str = "grammar-mcp.";

/// A Channel inbox's name prefix, the host's nom or a random suffix after it.
const CHANNEL_PREFIX: &str = "grammar.channel.";

/// A scratch directory's mode: its owner's alone.
const SCRATCH_MODE: u32 = 0o700;

/// This process's scratch directories, made once at startup.
static SCRATCH: OnceLock<ScratchDirs> = OnceLock::new();

/// Every Channel inbox this process made, removed at its ordinary end.
static CHANNEL_DIRS: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

/// A process's two scratch directories.
#[derive(Debug, Clone)]
pub(crate) struct ScratchDirs {
    /// Bulk, under the temporary directory: a definition's `$env.EQUIP_TMP`.
    pub(crate) tmp: PathBuf,
    /// Small runtime files, under the runtime directory: `$env.EQUIP_TRANSIENT`.
    pub(crate) transient: PathBuf,
}

/// Make this process's scratch directories, a dead process's swept first.
pub(crate) fn make_scratch_dirs() -> Result<ScratchDirs, String> {
    if let Some(dirs) = SCRATCH.get() {
        return Ok(dirs.clone());
    }
    let tmp_base = tmp_base();
    let transient_base = runtime_base().unwrap_or_else(|| {
        eprintln!(
            "grammar: neither UENV_USR_TRANSIENT nor XDG_RUNTIME_DIR names a directory; \
             transient files go under {}",
            tmp_base.display(),
        );
        tmp_base.clone()
    });
    sweep_stale(&tmp_base);
    if transient_base != tmp_base {
        sweep_stale(&transient_base);
    }
    let tmp = make_dir(&tmp_base)?;
    let transient = make_dir(&transient_base).inspect_err(|_| {
        let _ = fs::remove_dir_all(&tmp);
    })?;
    Ok(SCRATCH.get_or_init(|| ScratchDirs { tmp, transient }).clone())
}

/// This process's bulk scratch directory, once `make_scratch_dirs` has made it.
pub(crate) fn tmp_dir() -> Option<&'static Path> {
    SCRATCH.get().map(|dirs| dirs.tmp.as_path())
}

/// This process's transient scratch directory, once made.
pub(crate) fn transient_dir() -> Option<&'static Path> {
    SCRATCH.get().map(|dirs| dirs.transient.as_path())
}

/// Make a Channel inbox beside the transient directory, named for `nom`.
pub(crate) fn make_channel_dir(nom: &str) -> Result<PathBuf, String> {
    let base = transient_dir()
        .and_then(Path::parent)
        .ok_or_else(|| "the host has no scratch directory".to_string())?;
    let named = base.join(format!("{CHANNEL_PREFIX}{nom}"));
    let dir = match fs::DirBuilder::new().mode(SCRATCH_MODE).create(&named) {
        Ok(()) => named,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => tempfile::Builder::new()
            .prefix(CHANNEL_PREFIX)
            .permissions(fs::Permissions::from_mode(SCRATCH_MODE))
            .tempdir_in(base)
            .map_err(|e| format!("make a Channel inbox in {}: {e}", base.display()))?
            .keep(),
        Err(e) => return Err(format!("make {}: {e}", named.display())),
    };
    CHANNEL_DIRS.lock().unwrap_or_else(|e| e.into_inner()).push(dir.clone());
    Ok(dir)
}

/// Remove a Channel inbox `make_channel_dir` made, and forget it.
pub(crate) fn remove_channel_dir(dir: &Path) {
    CHANNEL_DIRS.lock().unwrap_or_else(|e| e.into_inner()).retain(|made| made != dir);
    remove_dir(dir);
}

/// Remove this process's scratch directories and Channel inboxes at its end.
pub(crate) fn remove_scratch_dirs() {
    let inboxes = std::mem::take(&mut *CHANNEL_DIRS.lock().unwrap_or_else(|e| e.into_inner()));
    for dir in &inboxes {
        remove_dir(dir);
    }
    let Some(dirs) = SCRATCH.get() else {
        return;
    };
    for dir in [&dirs.tmp, &dirs.transient] {
        remove_dir(dir);
    }
}

/// Remove `dir` whole, one already gone no error.
fn remove_dir(dir: &Path) {
    if let Err(e) = fs::remove_dir_all(dir)
        && e.kind() != io::ErrorKind::NotFound
    {
        eprintln!("grammar: remove {}: {e}", dir.display());
    }
}

/// The temporary directory: `UENV_USR_TMP`, else `TMPDIR`, else `/tmp`.
fn tmp_base() -> PathBuf {
    home_var("UENV_USR_TMP").unwrap_or_else(|| PathBuf::from(TMP_DEFAULT))
}

/// The runtime directory: `UENV_USR_TRANSIENT`, else `XDG_RUNTIME_DIR`.
fn runtime_base() -> Option<PathBuf> {
    home_var("UENV_USR_TRANSIENT")
}

/// One scratch directory in `base`, mode 0700 from creation, its owner marked.
fn make_dir(base: &Path) -> Result<PathBuf, String> {
    let made = match sourcetrait_grammar_scratch::Owner::current() {
        Ok(owner) => {
            sourcetrait_grammar_scratch::OwnedDir::create(base, SCRATCH_PREFIX, &owner).map(|dir| dir.path().to_path_buf())
        }
        Err(_) => tempfile::Builder::new()
            .prefix(SCRATCH_PREFIX)
            .permissions(fs::Permissions::from_mode(SCRATCH_MODE))
            .tempdir_in(base)
            .map(|dir| dir.keep()),
    };
    made.map_err(|e| format!("make a scratch directory in {}: {e}", base.display()))
}

/// Sweep `base` of scratch whose owner is proven gone, reporting what it keeps.
fn sweep_stale(base: &Path) {
    let report = sourcetrait_grammar_scratch::sweep(
        base,
        SCRATCH_PREFIX,
        sourcetrait_grammar_scratch::current_uid(),
        &sourcetrait_grammar_scratch::HostProbe,
    );
    for line in report.diagnostics() {
        eprintln!("grammar: {line}");
    }
}
