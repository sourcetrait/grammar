//! The guard: a marker read, an owner's liveness proven, and the sweep's verdicts.

use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use sourcetrait_grammar_scratch::*;

/// The boot and namespace every fake marker and probe share.
const BOOT: &str = "boot-a";
const PIDNS: &str = "pid:[4026531836]";

/// A probe answering what its test sets.
struct FakeProbe {
    answer: ProcessAnswer,
    boot: Option<String>,
    pidns: Option<String>,
    keeps: bool,
}

impl Probe for FakeProbe {
    fn process(&self, _pid: u32) -> ProcessAnswer {
        self.answer
    }

    fn boot(&self) -> Option<String> {
        self.boot.clone()
    }

    fn pid_namespace(&self) -> Option<String> {
        self.pidns.clone()
    }

    fn keeps_pid_namespaces(&self) -> bool {
        self.keeps
    }
}

/// A namespacing probe of the fakes' boot and namespace answering `answer`.
fn probe(answer: ProcessAnswer) -> FakeProbe {
    FakeProbe { answer, boot: Some(BOOT.to_string()), pidns: Some(PIDNS.to_string()), keeps: true }
}

fn started(start: u64, zombie: bool) -> ProcessAnswer {
    ProcessAnswer::Started { start, zombie }
}

/// An owner of this UID at `pid`, started at 100 in the fakes' boot and namespace.
fn owner(pid: u32) -> Owner {
    Owner {
        uid: current_uid(),
        pid,
        start: 100,
        boot: Some(BOOT.to_string()),
        pidns: Some(PIDNS.to_string()),
        nonce: Some("ab".repeat(16)),
    }
}

/// A marker's NUON for `uid`'s process `pid`, started at 100 in the fakes' boot.
fn nuon(uid: u32, pid: u32) -> String {
    format!("{{uid: {uid}, pid: {pid}, start: 100, boot: {BOOT:?}, pidns: {PIDNS:?}, nonce: {:?}}}", "ab".repeat(16))
}

/// A directory `name` under `base`, 0700, its marker `file` holding any `text`.
fn marked(base: &Path, name: &str, file: &str, text: Option<&str>) -> PathBuf {
    let dir = base.join(name);
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    if let Some(text) = text {
        std::fs::write(dir.join(file), text).unwrap();
    }
    dir
}

#[test]
fn an_owner_is_dead_only_on_proof() {
    let marked = owner(4242);
    let dead = |answer| matches!(liveness(&marked, &probe(answer)), Liveness::Dead(_));
    let unknown = |owner: &Owner, probe: &FakeProbe| matches!(liveness(owner, probe), Liveness::Unknown(_));
    assert_eq!(liveness(&marked, &probe(started(100, false))), Liveness::Alive);
    assert!(dead(started(100, true)), "a zombie has exited");
    assert!(dead(started(250, false)), "its pid reused");
    assert!(dead(ProcessAnswer::Absent), "no process at its pid");
    assert!(unknown(&marked, &probe(ProcessAnswer::Denied)), "a refused signal");
    assert!(unknown(&marked, &probe(ProcessAnswer::Unreadable)), "a process that can't be read");
    let another_boot = FakeProbe { boot: Some("boot-b".to_string()), ..probe(ProcessAnswer::Absent) };
    assert!(unknown(&marked, &another_boot), "another boot, an earlier one or another machine's");
    let no_boot = FakeProbe { boot: None, ..probe(ProcessAnswer::Absent) };
    assert!(unknown(&marked, &no_boot), "a boot the probe can't read");
    let another_pidns = FakeProbe { pidns: Some("pid:[4026532000]".to_string()), ..probe(ProcessAnswer::Absent) };
    assert!(unknown(&marked, &another_pidns), "another PID namespace's pid");
    let legacy = Owner { boot: None, pidns: None, nonce: None, ..owner(4242) };
    assert!(unknown(&legacy, &probe(ProcessAnswer::Absent)), "a predecessor's marker names no boot");
    let unnamespaced = Owner { pidns: None, ..owner(4242) };
    let platform_without = FakeProbe { pidns: None, keeps: false, ..probe(ProcessAnswer::Absent) };
    assert!(
        matches!(liveness(&unnamespaced, &platform_without), Liveness::Dead(_)),
        "where the platform keeps no PID namespace, its pid is judged",
    );
    assert!(unknown(&marked, &platform_without), "a namespace named where the platform keeps none");
    let unread = FakeProbe { pidns: None, ..probe(ProcessAnswer::Absent) };
    assert!(unknown(&unnamespaced, &unread), "both namespaces unread on a platform that keeps them");
    assert!(unknown(&marked, &unread), "this one unread");
}

#[test]
fn the_host_probe_proves_this_process_alive_and_a_reaped_child_dead() {
    let me = Owner::current().expect("this process as an owner");
    assert!(me.boot.is_some(), "the platform's boot is read");
    assert_eq!(liveness(&me, &HostProbe), Liveness::Alive);
    let reused = Owner { start: me.start - 1, ..me.clone() };
    assert!(matches!(liveness(&reused, &HostProbe), Liveness::Dead(_)), "this pid under another start");
    let mut child = std::process::Command::new("true").spawn().expect("a child");
    let pid = child.id();
    child.wait().expect("reaped");
    let gone = Owner { pid, ..me };
    assert!(matches!(liveness(&gone, &HostProbe), Liveness::Dead(_)), "a reaped child's pid");
}

#[test]
fn a_marker_reads_back_and_a_predecessors_takes_its_directorys_uid() {
    let root = tempfile::tempdir().unwrap();
    let me = Owner::current().unwrap();
    let dir = OwnedDir::create(root.path(), "t.", &me).expect("an owned directory");
    assert_eq!(Owner::read(dir.path(), current_uid()), Ok(me));
    assert_eq!(std::fs::metadata(dir.path()).unwrap().permissions().mode() & 0o777, 0o700);
    let legacy = marked(root.path(), "t.legacy", "owner", Some("4242 100\n"));
    assert_eq!(
        Owner::read(&legacy, 7),
        Ok(Owner { uid: 7, pid: 4242, start: 100, boot: None, pidns: None, nonce: None }),
    );
}

#[test]
fn a_malformed_or_unplain_marker_names_no_owner() {
    let root = tempfile::tempdir().unwrap();
    let base = root.path();
    let uid = current_uid();
    assert_eq!(Owner::read(&marked(base, "absent", "owner.nuon", None), uid), Err(MarkerError::Absent));
    let malformed = [
        ("partial", "owner.nuon", "{uid: ".to_string()),
        ("shape", "owner.nuon", "{uid: 1}".to_string()),
        ("pid_zero", "owner.nuon", nuon(uid, 0)),
        ("pid_wide", "owner.nuon", nuon(uid, u32::MAX)),
        ("pid_negative", "owner.nuon", nuon(uid, 0).replace("pid: 0", "pid: -1")),
        ("oversize", "owner.nuon", " ".repeat(5000)),
        ("legacy_words", "owner", "a b".to_string()),
        ("legacy_three", "owner", "1 2 3".to_string()),
        ("legacy_pid_zero", "owner", "0 5".to_string()),
    ];
    for (name, file, text) in malformed {
        let read = Owner::read(&marked(base, name, file, Some(&text)), uid);
        assert!(matches!(read, Err(MarkerError::Malformed(_))), "{name}: {read:?}");
    }
    let target = base.join("target.nuon");
    std::fs::write(&target, nuon(uid, 4242)).unwrap();
    let linked = marked(base, "linked", "owner.nuon", None);
    symlink(&target, linked.join("owner.nuon")).unwrap();
    let fifo = marked(base, "fifo", "owner.nuon", None);
    nix::unistd::mkfifo(&fifo.join("owner.nuon"), nix::sys::stat::Mode::S_IRWXU).unwrap();
    let nested = marked(base, "nested", "owner.nuon", None);
    std::fs::create_dir(nested.join("owner.nuon")).unwrap();
    for dir in [linked, fifo, nested] {
        let read = Owner::read(&dir, uid);
        assert!(matches!(read, Err(MarkerError::Unreadable(_))), "{}: {read:?}", dir.display());
    }
}

#[test]
fn the_sweep_removes_a_proven_dead_owners_directory_alone() {
    let root = tempfile::tempdir().unwrap();
    let base = root.path();
    let uid = current_uid();
    let dead = marked(base, "g.dead", "owner.nuon", Some(&nuon(uid, 4242)));
    let legacy = marked(base, "g.legacy", "owner", Some("4243 100\n"));
    let unmarked = marked(base, "g.unmarked", "owner.nuon", None);
    let malformed = marked(base, "g.malformed", "owner.nuon", Some("{uid: "));
    let foreign = marked(base, "g.foreign", "owner.nuon", Some(&nuon(uid.wrapping_add(1), 4244)));
    let other = marked(base, "h.dead", "owner.nuon", Some(&nuon(uid, 4245)));
    let file = base.join("g.file");
    std::fs::write(&file, "").unwrap();
    let target = marked(base, "elsewhere", "owner.nuon", Some(&nuon(uid, 4246)));
    let link = base.join("g.link");
    symlink(&target, &link).unwrap();
    let report = sweep(base, "g.", uid, &probe(ProcessAnswer::Absent));
    assert_eq!(report.removed, vec![dead.clone()], "the dead owner's directory goes");
    assert!(!dead.exists());
    for stays in [&legacy, &unmarked, &malformed, &foreign, &other, &file, &target] {
        assert!(stays.exists(), "{} stays", stays.display());
    }
    assert!(link.symlink_metadata().is_ok(), "the symlink stays");
    let kept: Vec<&Path> = report.kept.iter().map(|kept| kept.path.as_path()).collect();
    for reported in [&legacy, &unmarked, &malformed, &foreign, &file, &link] {
        assert!(kept.contains(&reported.as_path()), "{} is reported", reported.display());
    }
    assert!(!kept.contains(&other.as_path()), "another prefix's is never judged");
    assert_eq!(report.diagnostics().len(), 6, "{:?}", report.diagnostics());
}

#[test]
fn the_sweep_keeps_what_another_uid_owns_and_what_it_cant_prove() {
    let root = tempfile::tempdir().unwrap();
    let base = root.path();
    let uid = current_uid();
    let dir = marked(base, "g.held", "owner.nuon", Some(&nuon(uid, 4242)));
    let report = sweep(base, "g.", uid.wrapping_add(1), &probe(ProcessAnswer::Absent));
    assert!(dir.exists(), "another UID's directory stays");
    assert!(report.kept[0].reason.contains("owns it"), "{:?}", report.kept);
    for answer in [ProcessAnswer::Denied, ProcessAnswer::Unreadable] {
        let report = sweep(base, "g.", uid, &probe(answer));
        assert!(dir.exists(), "{answer:?} proves nothing");
        assert_eq!(report.diagnostics().len(), 1, "and is reported");
    }
    let elsewhere = FakeProbe { pidns: Some("pid:[4026532000]".to_string()), ..probe(ProcessAnswer::Absent) };
    let report = sweep(base, "g.", uid, &elsewhere);
    assert!(dir.exists(), "an owner in another PID namespace stays");
    assert_eq!(report.diagnostics().len(), 1, "and is reported");
    let report = sweep(base, "g.", uid, &probe(started(100, false)));
    assert!(dir.exists(), "a running owner's stays");
    assert!(report.diagnostics().is_empty(), "unreported");
}

#[test]
fn a_guard_removes_its_own_directory_alone() {
    let root = tempfile::tempdir().unwrap();
    let me = Owner::current().unwrap();
    let swapped = OwnedDir::create(root.path(), "g.", &me).unwrap();
    let marker = swapped.path().join("owner.nuon");
    std::fs::remove_file(&marker).unwrap();
    std::fs::write(&marker, nuon(me.uid, me.pid).replace(&"ab".repeat(16), &"cd".repeat(16))).unwrap();
    assert!(swapped.remove().is_err(), "a marker another guard's");
    assert!(swapped.path().is_dir(), "is left");
    let mine = OwnedDir::create(root.path(), "g.", &Owner::current().unwrap()).unwrap();
    mine.remove().expect("its own");
    assert!(!mine.path().exists());
    mine.remove().expect("one already gone is no error");
}
