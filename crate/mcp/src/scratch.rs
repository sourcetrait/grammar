//! The host's own scratch: its bulk directory, and its tree under the runtime one.
use crate::*;
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

/// A bulk scratch directory's name prefix, which the sweep matches.
const SCRATCH_PREFIX: &str = "grammar-mcp.";

/// The directory under the runtime one holding each host's tree, named for its nom.
const HOST_TREES: &str = "grammar";

/// The Channel's directory in the host's tree.
const CHANNEL_DIR: &str = "channel";

/// A scratch directory's mode on Unix, its owner's alone.
#[cfg(unix)]
const SCRATCH_MODE: u32 = 0o700;

/// This process's scratch, made once at startup.
static SCRATCH: OnceLock<ScratchDirs> = OnceLock::new();

/// A process's bulk directory and its tree.
#[derive(Debug, Clone)]
pub(crate) struct ScratchDirs {
    /// Bulk, under the temporary directory: a definition's `$env.EQUIP_TMP`.
    pub(crate) tmp: PathBuf,
    /// The host's tree, `grammar/<nom>/` under the runtime directory:
    /// `$env.EQUIP_TRANSIENT`, the Channel's `channel/` within it.
    pub(crate) transient: PathBuf,
}

/// Make this process's scratch, its tree named for `nom`, dead hosts' swept first.
pub(crate) fn make_scratch_dirs(nom: &str) -> Result<ScratchDirs, String> {
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
    let trees = transient_base.join(HOST_TREES);
    sweep_stale(&tmp_base, SCRATCH_PREFIX);
    if transient_base != tmp_base {
        sweep_stale(&transient_base, SCRATCH_PREFIX);
    }
    make_private(&trees).map_err(|e| format!("make {}: {e}", trees.display()))?;
    sweep_stale(&trees, "");
    let tmp = make_dir(&tmp_base)?;
    let transient = make_tree(&trees, nom).inspect_err(|_| {
        let _ = fs::remove_dir_all(&tmp);
    })?;
    Ok(SCRATCH.get_or_init(|| ScratchDirs { tmp, transient }).clone())
}

/// This process's bulk scratch directory, once `make_scratch_dirs` has made it.
pub(crate) fn tmp_dir() -> Option<&'static Path> {
    SCRATCH.get().map(|dirs| dirs.tmp.as_path())
}

/// This process's tree, once made.
pub(crate) fn transient_dir() -> Option<&'static Path> {
    SCRATCH.get().map(|dirs| dirs.transient.as_path())
}

/// The Channel's directory in the host's tree, made private where absent.
///
/// One left by an earlier open's teardown is taken as it stands, the packets'
/// ids keeping its files apart.
pub(crate) fn make_channel_dir() -> Result<PathBuf, String> {
    let tree = transient_dir().ok_or_else(|| "the host has no scratch directory".to_string())?;
    let dir = tree.join(CHANNEL_DIR);
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    builder.mode(SCRATCH_MODE);
    match builder.create(&dir) {
        Ok(()) => Ok(dir),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists && is_dir(&dir) => Ok(dir),
        Err(e) => Err(format!("make {}: {e}", dir.display())),
    }
}

/// Remove the Channel's directory, a deliberate close's.
pub(crate) fn remove_channel_dir(dir: &Path) {
    remove_dir(dir);
}

/// Remove this process's scratch at its end, its tree with the Channel's in it.
pub(crate) fn remove_scratch_dirs() {
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

/// Whether `path` is a directory itself, a link to one not counting.
fn is_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir())
}

/// The temporary directory: `UENV_USR_TMP`, else `TMPDIR`, else the platform's.
fn tmp_base() -> PathBuf {
    home_var("UENV_USR_TMP").unwrap_or_else(tmp_default)
}

/// The runtime directory: `UENV_USR_TRANSIENT`, else `XDG_RUNTIME_DIR`.
fn runtime_base() -> Option<PathBuf> {
    home_var("UENV_USR_TRANSIENT")
}

/// `dir` and any parent it lacks, each made private.
fn make_private(dir: &Path) -> io::Result<()> {
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    builder.mode(SCRATCH_MODE);
    builder.create(dir)
}

/// One bulk directory in `base` under the sweep's prefix, its owner marked.
fn make_dir(base: &Path) -> Result<PathBuf, String> {
    let made = match sourcetrait_grammar_scratch::Owner::current() {
        Ok(owner) => {
            sourcetrait_grammar_scratch::OwnedDir::create(base, SCRATCH_PREFIX, &owner).map(|dir| dir.path().to_path_buf())
        }
        Err(_) => private_tempdir(base, SCRATCH_PREFIX),
    };
    made.map_err(|e| format!("make a scratch directory in {}: {e}", base.display()))
}

/// The host's tree, `trees/<nom>/`, private from creation and its owner marked.
fn make_tree(trees: &Path, nom: &str) -> Result<PathBuf, String> {
    let made = match sourcetrait_grammar_scratch::Owner::current() {
        Ok(owner) => {
            sourcetrait_grammar_scratch::OwnedDir::create_named(trees, nom, &owner).map(|dir| dir.path().to_path_buf())
        }
        Err(_) => make_named(trees, nom),
    };
    made.map_err(|e| format!("make the host's tree {}: {e}", trees.join(nom).display()))
}

/// A private directory at `base/name`, refused where the name is taken.
fn make_named(base: &Path, name: &str) -> io::Result<PathBuf> {
    let path = base.join(name);
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    builder.mode(SCRATCH_MODE);
    builder.create(&path)?;
    Ok(path)
}

/// A kept directory in `base` named `prefix` and a random suffix.
fn private_tempdir(base: &Path, prefix: &str) -> io::Result<PathBuf> {
    let mut builder = tempfile::Builder::new();
    builder.prefix(prefix);
    #[cfg(unix)]
    builder.permissions(fs::Permissions::from_mode(SCRATCH_MODE));
    builder.tempdir_in(base).map(|dir| dir.keep())
}

/// Sweep `base` of `prefix`'s scratch whose owner is proven gone, reporting what it keeps.
fn sweep_stale(base: &Path, prefix: &str) {
    let report = sourcetrait_grammar_scratch::sweep(
        base,
        prefix,
        sourcetrait_grammar_scratch::current_uid(),
        &sourcetrait_grammar_scratch::HostProbe,
    );
    for line in report.diagnostics() {
        eprintln!("grammar: {line}");
    }
}
