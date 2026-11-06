//! The host's scratch directories and Channel inboxes, and their lifecycle.
//!
//! The scratch directories are private to their process, exported to every
//! definition, removed at an ordinary end, and swept once their owner is gone;
//! each Channel open makes an inbox beside the transient one.

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
use nix::sys::signal::Signal;
use sourcetrait_grammar_tests::{Host, NuVal, envelope, has_error_path, run_output_env};
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// A test's own directory, private as the platform's are: its mode 0700 on
/// Unix, its parent's access on Windows.
fn private_dir(dir: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&dir).expect("a base directory");
    #[cfg(unix)]
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
        .expect("the base's mode");
    dir
}

/// A test's own temporary and runtime directories.
fn bases(root: &Path) -> (PathBuf, PathBuf) {
    (private_dir(root.join("tmp")), private_dir(root.join("run")))
}

/// A path as the env a host is spawned with takes it.
fn text(path: &Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}

/// A host whose UENV homes name `tmp` and `run`, the standard ones decoys.
fn spawn_on(scratch: &Path, id: &str, tmp: &Path, run: &Path) -> Host {
    let decoy_tmp = private_dir(scratch.join("decoy").join("tmp"));
    let decoy_run = private_dir(scratch.join("decoy").join("run"));
    Host::spawn_full(
        scratch,
        &["--id", id],
        &[
            ("UENV_USR_TMP", text(tmp)),
            ("UENV_USR_TRANSIENT", text(run)),
            ("TMPDIR", text(&decoy_tmp)),
            ("XDG_RUNTIME_DIR", text(&decoy_run)),
        ],
    )
}

/// The host's bulk and transient directories, as `interface info` reports them.
fn scratch_dirs(host: &mut Host) -> (PathBuf, PathBuf) {
    let info = host.info();
    let result = &envelope(&info)["result"];
    let path = |value: &NuVal, name: &str| {
        PathBuf::from(value.as_str().unwrap_or_else(|| panic!("info carries {name}; got {info}")))
    };
    (path(&result["tmp"], "tmp"), path(&result["transient"]["grammar"], "transient.grammar"))
}

/// An agent's transient scratch, as `interface info` reports it: `transient.ai`.
fn ai_dir(host: &mut Host) -> PathBuf {
    let info = host.info();
    PathBuf::from(
        envelope(&info)["result"]["transient"]["ai"]
            .as_str()
            .unwrap_or_else(|| panic!("info carries transient.ai; got {info}")),
    )
}

/// The scratch directories `base` holds.
fn scratch_in(base: &Path) -> usize {
    std::fs::read_dir(base)
        .expect("the base reads")
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("grammar-mcp."))
        .count()
}

/// The inbox a sync Channel opens on, as the open answers it.
fn open_inbox(host: &mut Host) -> PathBuf {
    let resp = host.interface("interface channel open sync");
    PathBuf::from(
        envelope(&resp)["result"]["inbox"]
            .as_str()
            .unwrap_or_else(|| panic!("the open answers its inbox; got {resp}")),
    )
}

/// A directory's permission bits.
#[cfg(unix)]
fn mode(dir: &Path) -> u32 {
    std::fs::metadata(dir).expect("the directory exists").permissions().mode() & 0o777
}

/// A private directory in `base`, under the sweep's prefix, marked by its host,
/// its mode held on Unix.
fn assert_scratch(dir: &Path, base: &Path, pid: u32) {
    assert_eq!(dir.parent(), Some(base), "{} sits in {}", dir.display(), base.display());
    let meta = std::fs::metadata(dir).expect("the scratch directory exists");
    assert!(meta.is_dir(), "{} is a directory", dir.display());
    #[cfg(unix)]
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o700,
        "only its owner may enter {}",
        dir.display(),
    );
    let name = dir.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    assert!(name.starts_with("grammar-mcp."), "the sweep matches its prefix; got {name}");

    let owner = std::fs::read_to_string(dir.join("owner.nuon")).expect("its owner marker");
    assert!(owner.contains(&format!("pid: {pid},")), "the marker names its host's pid; got {owner:?}");
    assert!(owner.contains("nonce: "), "and its guard's nonce; got {owner:?}");
}

/// The host's tree, `grammar/<nom>/` in `base`, private and marked by its host.
fn assert_tree(dir: &Path, base: &Path, pid: u32) {
    let trees = base.join("grammar");
    assert_eq!(dir.parent(), Some(trees.as_path()), "{} sits in {}", dir.display(), trees.display());
    let name = dir.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    assert!(
        !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric()),
        "the tree is named for the host's nom; got {name}",
    );
    #[cfg(unix)]
    assert_eq!(mode(dir), 0o700, "only its owner may enter {}", dir.display());
    let owner = std::fs::read_to_string(dir.join("owner.nuon")).expect("its owner marker");
    assert!(owner.contains(&format!("pid: {pid},")), "the marker names its host's pid; got {owner:?}");
}

/// The UENV homes come first, so the standard variables' decoys stay empty.
#[tested]
fn both_directories_are_private_marked_and_exported() {
    let t = testing::test!({ .using_temp_dir() });
    let (tmp, run) = bases(t.temp_dir());
    let scratch = t.temp_dir().join("host");
    let mut host = spawn_on(&scratch, "scratchown", &tmp, &run);
    let (tmp_dir, transient_dir) = scratch_dirs(&mut host);
    assert_scratch(&tmp_dir, &tmp, host.pid());
    assert_tree(&transient_dir, &run, host.pid());
    let ai = ai_dir(&mut host);
    assert_eq!(ai, run.join("ai").join("scratchown"), "an agent's scratch sits under the runtime base by the id");
    assert!(!ai.exists(), "the host reports it and makes nothing there");
    for decoy in ["tmp", "run"] {
        let decoy = scratch.join("decoy").join(decoy);
        assert_eq!(scratch_in(&decoy), 0, "nothing in {}", decoy.display());
    }

    let resp = host.execute(
        "def execute [data: nothing]: nothing -> record<tmp: string, transient: string> { \
         { tmp: $env.EQUIP_TMP, transient: $env.EQUIP_TRANSIENT } }",
        "null",
    );
    let result = &envelope(&resp)["result"];
    assert_eq!(
        result["tmp"].as_str(),
        tmp_dir.to_str(),
        "a definition reads the bulk one as $env.EQUIP_TMP; got {resp}",
    );
    assert_eq!(
        result["transient"].as_str(),
        transient_dir.to_str(),
        "and the transient one as $env.EQUIP_TRANSIENT; got {resp}",
    );
}

/// With no UENV homes, `TMPDIR` and `XDG_RUNTIME_DIR` name the bases.
#[tested]
fn the_standard_variables_stand_behind_the_uenv_ones() {
    let t = testing::test!({ .using_temp_dir() });
    let (tmp, run) = bases(t.temp_dir());
    let mut host = Host::spawn_full(
        &t.temp_dir().join("host"),
        &["--id", "scratchstd"],
        &[
            ("UENV_USR_TMP", ""),
            ("UENV_USR_TRANSIENT", ""),
            ("TMPDIR", text(&tmp)),
            ("XDG_RUNTIME_DIR", text(&run)),
        ],
    );
    let (tmp_dir, transient_dir) = scratch_dirs(&mut host);
    assert_scratch(&tmp_dir, &tmp, host.pid());
    assert_tree(&transient_dir, &run, host.pid());
    assert_eq!(ai_dir(&mut host), run.join("ai").join("scratchstd"), "XDG_RUNTIME_DIR's, as the tree's");
}

/// With no runtime directory, transient files go under the temporary one.
///
/// The host warns of it, as the XDG spec asks.
#[tested]
fn transient_files_fall_back_to_the_temporary_directory() {
    let t = testing::test!({ .using_temp_dir() });
    let (tmp, _) = bases(t.temp_dir());
    let log = t.temp_dir().join("stderr.log");
    let mut host = Host::spawn_logged(
        &t.temp_dir().join("host"),
        &["--id", "scratchfall"],
        &[
            ("UENV_USR_TMP", text(&tmp)),
            ("UENV_USR_TRANSIENT", ""),
            ("XDG_RUNTIME_DIR", ""),
        ],
        &log,
    );
    let (tmp_dir, transient_dir) = scratch_dirs(&mut host);
    assert_scratch(&tmp_dir, &tmp, host.pid());
    assert_tree(&transient_dir, &tmp, host.pid());
    assert_ne!(tmp_dir, transient_dir, "two directories still");
    assert_eq!(ai_dir(&mut host), tmp.join("ai").join("scratchfall"), "an agent's scratch falls back with the tree");
    let inbox = open_inbox(&mut host);
    assert_eq!(inbox, transient_dir.join("channel"), "the Channel's directory goes in the tree");
    let stderr = std::fs::read_to_string(&log).expect("the host's stderr");
    assert!(stderr.contains("XDG_RUNTIME_DIR"), "the fallback is warned of; got {stderr:?}");
}

/// A temporary directory that doesn't exist fails the start.
///
/// The platform's directory is checked, never made.
#[tested]
fn a_missing_temporary_directory_fails_the_start() {
    let t = testing::test!({ .using_temp_dir() });
    let missing = t.temp_dir().join("absent");
    let out = run_output_env(
        &t.temp_dir().join("host"),
        &["--id", "scratchmiss", "cli", "interface", "interface info"],
        &[("UENV_USR_TMP", text(&missing))],
    );
    assert_eq!(out.status.code(), Some(2), "the start fails; got {out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(text(&missing)),
        "the refusal names {}; got {stderr}",
        missing.display(),
    );
    assert!(!missing.exists(), "{} is never made", missing.display());
}

/// A runtime directory that doesn't exist fails the start too.
///
/// The hosts' trees' parent is made in it, the base itself never, and the
/// failed start leaves no bulk directory behind.
#[tested]
fn a_missing_runtime_directory_fails_the_start() {
    let t = testing::test!({ .using_temp_dir() });
    let (tmp, _) = bases(t.temp_dir());
    let missing = t.temp_dir().join("absent");
    let out = run_output_env(
        &t.temp_dir().join("host"),
        &["--id", "scratchrunmiss", "cli", "interface", "interface info"],
        &[("UENV_USR_TMP", text(&tmp)), ("UENV_USR_TRANSIENT", text(&missing))],
    );
    assert_eq!(out.status.code(), Some(2), "the start fails; got {out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(text(&missing)),
        "the refusal names {}; got {stderr}",
        missing.display(),
    );
    assert!(!missing.exists(), "{} is never made", missing.display());
    assert_eq!(scratch_in(&tmp), 0, "no bulk directory is left in {}", tmp.display());
}

/// A trees' parent that isn't this user's alone fails the start.
///
/// One others may write would let them replace a host's tree, and a link
/// would put the trees wherever it points. Unix alone, whose modes it sets.
#[cfg(unix)]
#[tested]
fn a_trees_parent_not_this_users_alone_fails_the_start() {
    let t = testing::test!({ .using_temp_dir() });
    let (tmp, run) = bases(t.temp_dir());
    let trees = run.join("grammar");
    let start = |id: &str| {
        run_output_env(
            &t.temp_dir().join("host"),
            &["--id", id, "cli", "interface", "interface info"],
            &[("UENV_USR_TMP", text(&tmp)), ("UENV_USR_TRANSIENT", text(&run))],
        )
    };

    std::fs::create_dir(&trees).expect("a trees' parent");
    std::fs::set_permissions(&trees, std::fs::Permissions::from_mode(0o777)).expect("its mode");
    let out = start("scratchshared");
    assert_eq!(out.status.code(), Some(2), "an open parent fails the start; got {out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(text(&trees)), "the refusal names {}; got {stderr}", trees.display());
    assert_eq!(std::fs::read_dir(&trees).expect("it reads").count(), 0, "no tree is made in it");

    std::fs::remove_dir(&trees).expect("the open parent removed");
    let elsewhere = private_dir(t.temp_dir().join("elsewhere"));
    std::os::unix::fs::symlink(&elsewhere, &trees).expect("a link in its place");
    let out = start("scratchlinked");
    assert_eq!(out.status.code(), Some(2), "a link fails the start; got {out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(text(&trees)), "the refusal names {}; got {stderr}", trees.display());
    assert_eq!(std::fs::read_dir(&elsewhere).expect("it reads").count(), 0, "nothing lands there");
    assert_eq!(scratch_in(&tmp), 0, "no bulk directory is left in {}", tmp.display());
}

/// Unix alone, whose signal it sends.
#[cfg(unix)]
#[tested]
fn sigterm_removes_both_directories() {
    let t = testing::test!({ .using_temp_dir() });
    let (tmp, run) = bases(t.temp_dir());
    let mut host = spawn_on(&t.temp_dir().join("host"), "scratchterm", &tmp, &run);
    let (tmp_dir, transient_dir) = scratch_dirs(&mut host);
    assert!(tmp_dir.is_dir(), "{} exists while its host runs", tmp_dir.display());
    assert!(transient_dir.is_dir(), "{} exists while its host runs", transient_dir.display());

    let status = host
        .signal_and_wait(Signal::SIGTERM, Duration::from_secs(15))
        .expect("a signalled host should exit rather than hang");
    assert_eq!(status.code(), Some(128 + Signal::SIGTERM as i32), "the handler ran");
    assert!(!tmp_dir.exists(), "the signal sweep removes {}", tmp_dir.display());
    assert!(!transient_dir.exists(), "and {}", transient_dir.display());
}

/// Each open makes the Channel's directory, `channel/`, in the host's tree.
///
/// It is private, a send's packet landing in it as `<id>.nuon` and its
/// attachment in `<id>/`; a close removes it, a reopen makes it again, and the
/// host's end removes its tree and nothing else. The mode and the signalled
/// end on Unix alone.
#[tested]
fn each_open_makes_the_channel_inbox_in_the_transient_base() {
    let t = testing::test!({ .using_temp_dir() });
    let (tmp, run) = bases(t.temp_dir());
    let mut host = spawn_on(&t.temp_dir().join("host"), "scratchinbox", &tmp, &run);
    let info = host.info();
    let nom = envelope(&info)["result"]["nom"]
        .as_str()
        .unwrap_or_else(|| panic!("info carries nom; got {info}"))
        .to_string();
    let named = run.join("grammar").join(&nom).join("channel");

    let inbox = open_inbox(&mut host);
    assert_eq!(inbox, named, "the Channel's directory sits in the tree named for the host's nom");
    #[cfg(unix)]
    assert_eq!(mode(&inbox), 0o700, "only its owner may enter {}", inbox.display());
    host.interface("interface channel sync");
    host.interface("interface channel ready");
    let resp = host.execute(
        "def execute [data: nothing]: nothing -> record<event_id: string> { \
         grmr channel send test/inbox {n: 1} {n: 2} }",
        "null",
    );
    let id = envelope(&resp)["result"]["event_id"]
        .as_str()
        .unwrap_or_else(|| panic!("the send answers its id; got {resp}"))
        .to_string();
    assert!(inbox.join(format!("{id}.nuon")).is_file(), "the packet lands as {id}.nuon");
    assert!(
        inbox.join(&id).join("attached.nuon").is_file(),
        "the attachment lands in the packet's own {id}/",
    );

    host.interface("interface channel close");
    assert!(!inbox.exists(), "a close removes {}", inbox.display());

    let stray = run.join("grammar").join("strayhost");
    std::fs::create_dir(&stray).expect("a directory no host made");
    let reopened = open_inbox(&mut host);
    assert_eq!(reopened, named, "a reopen makes the Channel's directory again");
    #[cfg(unix)]
    {
        assert_eq!(mode(&reopened), 0o700, "only its owner may enter {}", reopened.display());
        let tree = reopened.parent().expect("the tree").to_path_buf();
        let status = host
            .signal_and_wait(Signal::SIGTERM, Duration::from_secs(15))
            .expect("a signalled host should exit rather than hang");
        assert_eq!(status.code(), Some(128 + Signal::SIGTERM as i32), "the handler ran");
        assert!(!tree.exists(), "the host's end removes its tree {}", tree.display());
        assert!(stray.is_dir(), "and leaves {}, which it never made", stray.display());
    }
}

/// A teardown other than a deliberate close leaves the Channel's files.
///
/// A send before `ready` tears the Channel down, leaving the greeting's event;
/// an open that fails to start, here for want of an `[identity]`, leaves it
/// too, and the next open takes the directory as it stands.
#[tested]
fn a_teardown_leaves_the_channels_files_for_the_next_open() {
    let t = testing::test!({ .using_temp_dir() });
    let cwd = t.temp_dir().join("bare");
    std::fs::create_dir_all(&cwd).expect("a start directory with no config");
    let mut host = Host::spawn_in(&t.temp_dir().join("host"), &["--id", "scratchkeep"], &cwd);
    let inbox = open_inbox(&mut host);
    let events = || {
        let mut events: Vec<PathBuf> = std::fs::read_dir(&inbox)
            .expect("the Channel's directory reads")
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "nuon"))
            .collect();
        events.sort();
        events
    };
    let greeting = events();
    assert_eq!(greeting.len(), 1, "the open writes its greeting's event; got {greeting:?}");

    let early = host.execute(
        "def execute [data: nothing]: nothing -> record<event_id: string> { \
         grmr channel send test/early {n: 1} }",
        "null",
    );
    assert!(has_error_path(&early), "a send before ready is refused; got {early}");
    assert_eq!(events(), greeting, "the teardown leaves the greeting's event");

    let failed = host.interface("interface channel open async");
    assert!(has_error_path(&failed), "an open with no [identity] fails to start; got {failed}");
    assert_eq!(events(), greeting, "and so does the failed start");

    assert_eq!(open_inbox(&mut host), inbox, "the next open takes the directory as it stands");
    assert!(greeting.iter().all(|event| event.is_file()), "the greeting's event is still in it");
}

/// A dead host's directories go at the next start.
///
/// A host dropped is killed outright, which runs no exit hook, so its
/// directories outlive it until the next start sweeps them.
#[tested]
fn a_dead_hosts_directories_are_swept_at_start() {
    let t = testing::test!({ .using_temp_dir() });
    let (tmp, run) = bases(t.temp_dir());
    let mut live = spawn_on(&t.temp_dir().join("live"), "scratchlive", &tmp, &run);
    let (live_tmp, live_transient) = scratch_dirs(&mut live);
    let mut dead = spawn_on(&t.temp_dir().join("dead"), "scratchdead", &tmp, &run);
    let (dead_tmp, dead_transient) = scratch_dirs(&mut dead);
    drop(dead);
    assert!(dead_tmp.is_dir(), "a host killed outright leaves {}", dead_tmp.display());
    assert!(dead_transient.is_dir(), "and {}", dead_transient.display());

    let _next = spawn_on(&t.temp_dir().join("next"), "scratchnext", &tmp, &run);
    assert!(!dead_tmp.exists(), "the next start sweeps {}", dead_tmp.display());
    assert!(!dead_transient.exists(), "and {}", dead_transient.display());
    assert!(live_tmp.is_dir(), "and keeps the live host's {}", live_tmp.display());
    assert!(live_transient.is_dir(), "and {}", live_transient.display());
}
