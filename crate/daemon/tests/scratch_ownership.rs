//! The worker's scratch: private, marked, swept of dead owners, its own to remove.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use sourcetrait_grammar_daemon::*;
use sourcetrait_grammar_scratch::{Owner, current_uid};

/// The variable naming a holder child's scratch root.
const HOLDER_ROOT: &str = "GRAMMARD_SCRATCH_HOLDER_ROOT";

/// Homes under `root`, each made, a runtime home where `runtime` asks.
fn homes_at(root: &Path, runtime: bool) -> InstanceHomes {
    let home = |name: &str| {
        let path = root.join(name);
        std::fs::create_dir_all(&path).unwrap();
        path
    };
    InstanceHomes {
        data: home("data"),
        cache: home("cache"),
        tmp: home("tmp"),
        runtime: runtime.then(|| home("run")),
        diagnostics: Vec::new(),
    }
}

/// A holder's half: its scratch made under its environment's root, then held.
#[test]
#[ignore = "a holder child's half, which its parent runs"]
fn hold_scratch() {
    let Ok(root) = std::env::var(HOLDER_ROOT) else {
        return;
    };
    let scratch = WorkerScratch::create(&homes_at(Path::new(&root), true)).expect("the holder's scratch");
    let mut out = std::io::stdout().lock();
    writeln!(out, "holder_tmp={}", scratch.tmp().display()).unwrap();
    writeln!(out, "holder_transient={}", scratch.transient().display()).unwrap();
    out.flush().unwrap();
    drop(out);
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
}

/// A holder child over `root`, and the two directories it reports.
///
/// A report is found anywhere in its line: with one test thread, libtest writes
/// `test hold_scratch ... ` unended before the body runs, and the first report
/// lands after it.
fn spawn_holder(root: &Path) -> (Child, PathBuf, PathBuf) {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "hold_scratch", "--ignored", "--nocapture", "--test-threads=1"])
        .env(HOLDER_ROOT, root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("a holder");
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let (mut tmp, mut transient) = (None, None);
    while tmp.is_none() || transient.is_none() {
        let line = lines.next().expect("the holder reports both directories").unwrap();
        if let Some((_, path)) = line.split_once("holder_tmp=") {
            tmp = Some(PathBuf::from(path));
        }
        if let Some((_, path)) = line.split_once("holder_transient=") {
            transient = Some(PathBuf::from(path));
        }
    }
    std::thread::spawn(move || lines.for_each(drop));
    (child, tmp.unwrap(), transient.unwrap())
}

#[test]
fn the_worker_scratch_is_private_marked_and_its_own_to_remove() {
    let root = tempfile::tempdir().unwrap();
    let homes = homes_at(root.path(), true);
    let scratch = WorkerScratch::create(&homes).expect("the worker's scratch");
    assert!(scratch.diagnostics().is_empty(), "{:?}", scratch.diagnostics());
    let runtime = homes.runtime.clone().unwrap();
    let mut nonces = Vec::new();
    for (dir, base) in [(scratch.tmp(), &homes.tmp), (scratch.transient(), &runtime)] {
        assert_eq!(dir.parent(), Some(base.as_path()));
        assert!(dir.file_name().unwrap().to_str().unwrap().starts_with("grammard."));
        assert_eq!(std::fs::metadata(dir).unwrap().permissions().mode() & 0o777, 0o700);
        let owner = Owner::read(dir, current_uid()).expect("its marker");
        assert_eq!((owner.uid, owner.pid), (current_uid(), std::process::id()));
        nonces.push(owner.nonce.expect("its nonce"));
    }
    assert_ne!(nonces[0], nonces[1], "a nonce each");
    scratch.remove().expect("removed");
    assert!(!scratch.tmp().exists() && !scratch.transient().exists());
}

#[test]
fn without_a_runtime_home_transient_files_go_under_the_tmp_home() {
    let root = tempfile::tempdir().unwrap();
    let homes = homes_at(root.path(), false);
    let scratch = WorkerScratch::create(&homes).unwrap();
    assert_eq!(scratch.transient().parent(), Some(homes.tmp.as_path()));
    assert_ne!(scratch.tmp(), scratch.transient(), "two directories still");
    assert!(scratch.diagnostics().iter().any(|d| d.contains("no runtime home")), "{:?}", scratch.diagnostics());
    scratch.remove().unwrap();
}

#[test]
fn a_missing_base_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let mut homes = homes_at(root.path(), true);
    homes.tmp = root.path().join("gone");
    let refused = WorkerScratch::create(&homes).expect_err("no tmp home");
    assert_eq!(refused.kind().name(), "scratch::unavailable");
    assert_eq!(refused.exit_code(), 73);
}

#[test]
fn a_killed_owners_scratch_is_swept_and_a_live_ones_kept() {
    let root = tempfile::tempdir().unwrap();
    let (mut killed, killed_tmp, killed_transient) = spawn_holder(root.path());
    let (mut live, live_tmp, live_transient) = spawn_holder(root.path());
    killed.kill().unwrap();
    killed.wait().unwrap();
    assert!(killed_tmp.is_dir() && killed_transient.is_dir(), "a killed owner leaves its scratch");
    let homes = homes_at(root.path(), true);
    let next = WorkerScratch::create(&homes).unwrap();
    assert!(!killed_tmp.exists() && !killed_transient.exists(), "the next start sweeps it");
    assert!(live_tmp.is_dir() && live_transient.is_dir(), "and keeps a live owner's");
    assert!(next.diagnostics().is_empty(), "{:?}", next.diagnostics());
    drop(live.stdin.take());
    live.wait().unwrap();
    assert!(live_tmp.is_dir(), "an owner ending without its removal leaves its scratch");
    let after = WorkerScratch::create(&homes).unwrap();
    assert!(!live_tmp.exists() && !live_transient.exists(), "and the next start sweeps that too");
    assert!(next.tmp().is_dir() && next.transient().is_dir(), "while a running worker's stays");
    next.remove().unwrap();
    after.remove().unwrap();
}
