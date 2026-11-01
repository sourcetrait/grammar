use crate::*;

/// The environment variable that overrides the bundled plugin directory.
pub(crate) const PLUGIN_DIR_ENV: &str = "GRAMMAR_MCP_PLUGIN_DIR";

/// The registry's file name in the namespace's data dir.
const REGISTRY_FILE: &str = "plugin.msgpackz";

/// The file-name prefix nushell requires of a plugin binary.
const PLUGIN_PREFIX: &str = "nu_plugin_";

/// The plugin directory's kinds: nushell's core, the grammar's own, a rig's.
const PLUGIN_KINDS: [&str; 3] = ["core", "grammar", "rig"];

/// The cache subdir a registration writes its output to.
const REGISTRATION_LOG_DIR: &str = "plugins";

/// The registry this namespace owns, `<data dir>/plugin.msgpackz`, once set.
pub(crate) fn registry_path() -> Option<PathBuf> {
    Some(params()?.data_home.join(REGISTRY_FILE))
}

pub(crate) fn read_registry() -> Option<nu::PluginRegistryFile> {
    let mut file = fs::File::open(registry_path()?).ok()?;
    nu::PluginRegistryFile::read_from(&mut file, None).ok()
}

/// Last-modified time of the plugin registry file; None when it is absent.
pub(crate) fn registry_mtime() -> Option<SystemTime> {
    fs::metadata(registry_path()?).and_then(|m| m.modified()).ok()
}

/// The bundled plugin directory: the override, else `<exe dir>/../plugin`.
pub(crate) fn plugin_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(PLUGIN_DIR_ENV) {
        return Some(PathBuf::from(dir));
    }
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.parent()?.join("plugin"))
}

/// Each `nu_plugin_*` file under the plugin directory's kinds, canonicalized.
pub(crate) fn bundled_plugins() -> Vec<PathBuf> {
    let Some(dir) = plugin_dir() else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = PLUGIN_KINDS
        .iter()
        .filter_map(|kind| fs::read_dir(dir.join(kind)).ok())
        .flatten()
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(PLUGIN_PREFIX))
        })
        .filter_map(|path| dunce::canonicalize(&path).ok())
        .collect();
    found.sort();
    found
}

/// Register each bundled plugin the registry lacks or holds older than it.
pub fn ensure_plugin_registry() {
    let Some(registry) = registry_path() else {
        return;
    };
    let contents = read_registry();
    let registered_at = registry_mtime();
    let stale: Vec<PathBuf> = bundled_plugins()
        .into_iter()
        .filter(|bin| needs_registration(bin, contents.as_ref(), registered_at))
        .collect();
    if stale.is_empty() {
        return;
    }
    if let Err(e) = register_plugins(&registry, stale) {
        eprintln!("grammar: plugins not registered into {}: {e}", registry.display());
    }
}

/// Whether `bin` is missing from the registry or changed after it was written.
fn needs_registration(
    bin: &Path,
    contents: Option<&nu::PluginRegistryFile>,
    registered_at: Option<SystemTime>,
) -> bool {
    let listed = contents.is_some_and(|c| c.plugins.iter().any(|item| item.filename == bin));
    let changed_at = changed_time(bin);
    match (listed, registered_at, changed_at) {
        (true, Some(registered), Some(changed)) => changed > registered,
        _ => true,
    }
}

/// When `path` last changed here: its inode's ctime, which no copy can set.
#[cfg(unix)]
fn changed_time(path: &Path) -> Option<SystemTime> {
    let metadata = fs::metadata(path).ok()?;
    let seconds = u64::try_from(metadata.ctime()).unwrap_or(0);
    let nanos = u32::try_from(metadata.ctime_nsec()).unwrap_or(0);
    Some(UNIX_EPOCH + Duration::new(seconds, nanos))
}

/// When `path` last changed here: its NTFS change time, which no copy can set.
#[cfg(windows)]
fn changed_time(path: &Path) -> Option<SystemTime> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{FILE_BASIC_INFO, FileBasicInfo, GetFileInformationByHandleEx};
    /// 100 ns intervals from 1601, Windows' epoch, to 1970.
    const EPOCH_GAP: u64 = 116_444_736_000_000_000;
    let file = fs::File::open(path).ok()?;
    let mut info = FILE_BASIC_INFO {
        CreationTime: 0,
        LastAccessTime: 0,
        LastWriteTime: 0,
        ChangeTime: 0,
        FileAttributes: 0,
    };
    let read = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileBasicInfo,
            (&mut info as *mut FILE_BASIC_INFO).cast(),
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        )
    };
    if read == 0 {
        return None;
    }
    let ticks = u64::try_from(info.ChangeTime).ok()?.checked_sub(EPOCH_GAP)?;
    Some(UNIX_EPOCH + Duration::from_nanos(ticks.saturating_mul(100)))
}

/// Run nushell's own `plugin add` for each binary, into `registry`.
fn register_plugins(
    registry: &Path,
    bins: Vec<PathBuf>,
) -> Result<(), String> {
    let log_dir = cache_home().join(REGISTRATION_LOG_DIR);
    fs::create_dir_all(&log_dir).map_err(|e| format!("create {}: {e}", log_dir.display()))?;
    let config = path_nuon(registry)?;
    let mut source = String::new();
    for bin in &bins {
        let bin = path_nuon(bin)?;
        source.push_str(&format!("plugin add --plugin-config {config} {bin}\n"));
    }
    std::thread::Builder::new()
        .stack_size(EVAL_STACK_SIZE)
        .name("nu-plugin-register".to_string())
        .spawn(move || {
            let mut engine = startup(Mode::Stateful);
            let cancel = Arc::new(AtomicBool::new(false));
            eval_in_process(&mut engine, &log_dir, &source, cancel, false, None, None)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .map_err(|e| format!("spawn the registration thread: {e}"))?
        .join()
        .map_err(|_| "the registration thread panicked".to_string())?
}

/// A path as a NUON string, for rendering into a source line.
fn path_nuon(path: &Path) -> Result<String, String> {
    let value = nu::Value::string(path.to_string_lossy().into_owned(), nu::Span::unknown());
    nu::to_nuon(&nu::EngineState::new(), &value, nu::ToNuonConfig::default())
        .map_err(|e| e.to_string())
}

pub(crate) fn load_plugin_decls(engine_state: &mut nu::EngineState) {
    let Some(path) = registry_path() else {
        return;
    };
    engine_state.plugin_path = Some(path.clone());
    let Some(contents) = read_registry() else {
        return;
    };
    let mut working_set = nu::StateWorkingSet::new(engine_state);
    let error_count = nu::load_plugin_file(&mut working_set, &contents, None);
    if error_count > 0 {
        eprintln!(
            "grammar: {error_count} plugin(s) failed to load from {}; see preceding error reports",
            path.display(),
        );
    }
    let delta = working_set.render();
    let _ = engine_state.merge_delta(delta);
}

pub fn list_registered_plugins() -> Vec<PluginInfo> {
    read_registry()
        .map(|f| {
            f.plugins
                .iter()
                .map(|p| {
                    let version = match &p.data {
                        nu::PluginRegistryItemData::Valid { metadata, .. } => {
                            metadata.version.clone()
                        }
                        nu::PluginRegistryItemData::Invalid => None,
                    };
                    PluginInfo(p.name.clone(), version)
                })
                .collect()
        })
        .unwrap_or_default()
}
