//! The destruction half: the per-namespace host lock, and the signal handlers that run
//! the shutdown sweep. Both need a REAL process that can be signalled and killed, so
//! both are system tests.

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
use nix::sys::signal::Signal;
#[cfg(unix)]
use sourcetrait_grammar_tests::envelope;
use sourcetrait_grammar_tests::{Host, namespace_dir};
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// Try to take the namespace's host lock the way an outside watcher would: any process,
/// any language, one non-blocking exclusive lock.
///
/// TRUE means we ACQUIRED it, which is the reading "the owner is gone". FALSE means it
/// is held, which is "the owner is alive". The file closes immediately, so a successful
/// probe leaves nothing behind.
fn watcher_sees_host_gone(namespace: &Path) -> bool {
    let path = namespace.join("host.lock");
    let Ok(file) = std::fs::OpenOptions::new().read(true).write(true).open(&path) else {
        // No file at all is not "alive" - it is a namespace no host ever locked.
        return true;
    };
    file.try_lock().is_ok()
}

/// `watcher_sees_host_gone` within `wait`: Windows releases a dead holder's lock
/// in its own time, which its documentation leaves to the system's resources.
fn watcher_sees_host_gone_within(namespace: &Path, wait: std::time::Duration) -> bool {
    let deadline = std::time::Instant::now() + wait;
    loop {
        if watcher_sees_host_gone(namespace) {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

/// The lock's contents are read on Unix alone: Windows' lock covers the whole
/// file, so no other process reads it while it is held.
#[tested]
fn the_host_lock_is_held_while_alive_and_dropped_on_sigkill() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--id", "locktest", "--namespace", "default"]);
    let namespace = namespace_dir(host.data_home(), "locktest", "default");

    #[cfg_attr(not(unix), allow(unused_variables))]
    let info = host.info();
    // Identity is in the CONTENTS, for a reader that wants to know which host holds it.
    #[cfg(unix)]
    {
        let mcp_nom = envelope(&info)["result"]["nom"]
            .as_str()
            .expect("info carries the host's nom")
            .to_string();
        let pid = host.pid();
        let contents = std::fs::read_to_string(namespace.join("host.lock")).expect("read host.lock");
        assert!(
            contents.contains(&mcp_nom) && contents.contains(&pid.to_string()),
            "the lock file should name its holder; got {contents:?}",
        );
    }

    assert!(
        !watcher_sees_host_gone(&namespace),
        "a live host must hold its namespace lock, or every watcher reads it as dead",
    );

    // Drop kills and reaps. This is the death no exit hook can ever run on - the one
    // that orphans a background job's external child - so it is the case the lock exists
    // for, and the kernel dropping the lock is what keeps "gone" the default reading.
    drop(host);
    assert!(
        watcher_sees_host_gone_within(&namespace, std::time::Duration::from_secs(5)),
        "the lock must be released when the host dies, even when killed",
    );
}

/// Unix alone, whose signal it sends.
#[cfg(unix)]
#[tested]
fn sigterm_runs_the_sweep_and_exits_on_its_own_terms() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--id", "sigtest", "--namespace", "default"]);
    // A completed call proves the host is up and serving before it is signalled.
    let _ = host.info();

    let status = host
        .signal_and_wait(Signal::SIGTERM, Duration::from_secs(15))
        .expect("a signalled host should exit rather than hang");

    assert_eq!(
        status.code(),
        Some(128 + Signal::SIGTERM as i32),
        "the handler must sweep and exit 128+signo; a plain signal death reports \
         code() == None instead, which is how the two are told apart",
    );
    assert!(
        status.signal().is_none(),
        "exiting on our own terms means NOT dying from the signal's default action",
    );
}

/// Unix alone, whose signal it sends.
#[cfg(unix)]
#[tested]
fn sighup_is_handled_too() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--id", "huptest", "--namespace", "default"]);
    let _ = host.info();

    let status = host
        .signal_and_wait(Signal::SIGHUP, Duration::from_secs(15))
        .expect("a signalled host should exit rather than hang");

    assert_eq!(status.code(), Some(128 + Signal::SIGHUP as i32));
}
