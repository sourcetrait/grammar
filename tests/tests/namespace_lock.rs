//! One engine a namespace: a held lock refuses the server and the one-shot CLI
//! before either starts an engine, and the one-shot CLI holds the lock for the
//! length of its call. The holder is this test, through the lock any holder
//! takes, so a worker holding it is refused alike.
#![cfg(unix)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nix::fcntl::{Flock, FlockArg};
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// The namespace lock's path under a host's data home.
fn lock_path(data_home: &Path, id: &str) -> PathBuf {
    namespace_dir(data_home, id, "default").join("host.lock")
}

/// The lock taken by this test with a record of its own, as another holder takes it.
fn hold(path: &Path) -> Flock<std::fs::File> {
    std::fs::create_dir_all(path.parent().expect("the namespace")).expect("mkdir the namespace");
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .expect("open the lock");
    let mut held = Flock::lock(file, FlockArg::LockExclusiveNonblock).expect("the test takes the lock");
    held.write_all(b"{holder: test, pid: 1}\n").expect("the test's record");
    held
}

/// Whether anyone holds the lock at `path`, tried without blocking.
fn held_by_anyone(path: &Path) -> bool {
    let Ok(file) = std::fs::OpenOptions::new().read(true).write(true).open(path) else {
        return false;
    };
    Flock::lock(file, FlockArg::LockExclusiveNonblock).is_err()
}

#[tested]
fn a_held_namespace_refuses_the_server() {
    let t = testing::test!({ .using_temp_dir() });
    let scratch = t.temp_dir();
    let _held = hold(&lock_path(&scratch.join("data"), "held"));
    let out = run_output(scratch, &["--id", "held", "--namespace", "default"]);
    assert_eq!(out.status.code(), Some(1), "the server refuses to start: {out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("is held"), "{stderr}");
    assert!(stderr.contains("holder: test"), "the holder is named: {stderr}");
}

#[tested]
fn a_held_namespace_refuses_the_one_shot_cli() {
    let t = testing::test!({ .using_temp_dir() });
    let scratch = t.temp_dir();
    let _held = hold(&lock_path(&scratch.join("data"), "held"));
    let out = run_output(
        scratch,
        &["--id", "held", "--namespace", "default", "cli", "interface", "interface info"],
    );
    assert_eq!(out.status.code(), Some(1), "the one-shot refuses to start: {out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("is held"), "{stderr}");
    assert!(stdout_str(&out).is_empty(), "no envelope printed: {out:?}");
}

#[tested]
fn the_one_shot_cli_holds_the_lock_through_its_call() {
    let t = testing::test!({ .using_temp_dir() });
    let scratch = t.temp_dir().to_path_buf();
    let path = lock_path(&scratch.join("data"), "oneshot");
    let runner = std::thread::spawn({
        let scratch = scratch.clone();
        move || {
            run_output(
                &scratch,
                &[
                    "--id",
                    "oneshot",
                    "--namespace",
                    "default",
                    "cli",
                    "execute",
                    "def execute [data: nothing]: nothing -> nothing { sleep 3sec }",
                    "null",
                ],
            )
        }
    });
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut recorded = false;
    while Instant::now() < deadline && !runner.is_finished() {
        let record = std::fs::read_to_string(&path).unwrap_or_default();
        if record.contains("grammar-mcp") {
            recorded = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(recorded, "the one-shot recorded itself in the lock");
    assert!(held_by_anyone(&path), "the one-shot holds the lock during its call");
    let out = runner.join().expect("the one-shot ran");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(!held_by_anyone(&path), "its end released the lock");
}
