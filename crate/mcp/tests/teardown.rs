//! External teardown: a timed-out eval's external child is reaped (the timeout ->
//! cancel -> tree-kill path), and so is that child's own child. In-process:
//! `nu_timeout` drives the real dispatch, and the reap is asserted by scanning the
//! processes' command lines, from /proc on Linux and from `ps` on the other Unixes,
//! and on Windows the kernel's process records for a ping this process started. The
//! external must have been seen running first, so a reap is never asserted of one
//! that hadn't yet started.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sourcetrait_grammar_mcp::guts::{TestServer, error_kind};

/// Any `/proc/<pid>/cmdline` containing `marker` (the child is distinctively
/// argged so this finds ours, not an unrelated process).
#[cfg(target_os = "linux")]
fn any_cmdline_contains(marker: &str) -> bool {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return false;
    };
    for e in entries.flatten() {
        if let Ok(bytes) = std::fs::read(e.path().join("cmdline"))
            && String::from_utf8_lossy(&bytes).contains(marker)
        {
            return true;
        }
    }
    false
}

/// Any command line `ps` lists containing `marker`.
#[cfg(all(unix, not(target_os = "linux")))]
fn any_cmdline_contains(marker: &str) -> bool {
    let out = std::process::Command::new("ps")
        .args(["-A", "-o", "args="])
        .output()
        .expect("run ps");
    String::from_utf8_lossy(&out.stdout).lines().any(|line| line.contains(marker))
}

/// On Windows, a `ping` among this process's descendants, the in-process host's
/// externals being its children: a standard user's SSH logon is a network logon,
/// which WMI refuses, so no other process's command line is read, and the kernel's
/// process records give each one's name and parent. Each test's external is the one
/// ping running, the tests running one at a time.
#[cfg(windows)]
fn any_cmdline_contains(_marker: &str) -> bool {
    let table = sourcetrait_process::processes();
    sourcetrait_process::descendants_in(&table, std::process::id()).iter().any(|pid| {
        table.iter().any(|p| p.pid == *pid && p.name.eq_ignore_ascii_case("PING.EXE"))
    })
}

/// A call's timeout, longer than an eval takes to start its external on a loaded
/// host, so the external is running when the timeout fires.
const TIMEOUT: &str = "5sec";

/// How long a watcher looks for the external before giving up.
const WATCH: Duration = Duration::from_secs(30);

/// Watch for an external marked `marker` while a call runs: the flag holds once one
/// has been seen.
fn watch_for(marker: &'static str) -> (Arc<AtomicBool>, std::thread::JoinHandle<()>) {
    let seen = Arc::new(AtomicBool::new(false));
    let flag = seen.clone();
    let watcher = std::thread::spawn(move || {
        let deadline = Instant::now() + WATCH;
        while Instant::now() < deadline {
            if any_cmdline_contains(marker) {
                flag.store(true, Ordering::SeqCst);
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    (seen, watcher)
}

/// SIGKILL is asynchronous, so poll for the child to disappear.
fn wait_until_gone(marker: &str, secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        if !any_cmdline_contains(marker) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[test]
fn timed_out_external_is_reaped() {
    let s = TestServer::new();
    // The eval blocks on an external that runs for hours, `sleep 31337` on Unix and,
    // Windows having no sleep, a `ping` of 31337 echoes there; the timeout triggers
    // cancel + a tree-kill of the tracked external, which also unblocks the waiting
    // thread.
    let external = if cfg!(windows) { "^ping -n 31337 127.0.0.1" } else { "^sleep 31337" };
    let (seen, watcher) = watch_for("31337");
    let env = s.execute_timeout(
        &format!(
            "def execute [data: nothing]: nothing -> record<out: int> {{\n    {external}\n    {{ out: 0 }}\n}}"
        ),
        "null",
        Some(TIMEOUT),
    );
    assert_eq!(error_kind(&env), Some("thread::timeout"), "got {env}");
    watcher.join().expect("the watcher");
    assert!(seen.load(Ordering::SeqCst), "the external marked 31337 ran before the timeout");
    assert!(
        wait_until_gone("31337", 10),
        "the timed-out external marked 31337 should be reaped by the tree-kill",
    );
}

#[test]
fn timed_out_external_tree_is_reaped() {
    let s = TestServer::new();
    // The tracked external starts the long-running one as its own child, which only
    // the tree-kill's walk of the process table reaches: `xargs` starting
    // `sleep 31338` on Unix, PowerShell starting a `ping` of 31338 echoes on Windows.
    let external = if cfg!(windows) {
        "^powershell -NoProfile -NonInteractive -Command 'ping -n 31338 127.0.0.1'"
    } else {
        "'31338' | ^xargs sleep"
    };
    let (seen, watcher) = watch_for("31338");
    let env = s.execute_timeout(
        &format!(
            "def execute [data: nothing]: nothing -> record<out: int> {{\n    {external}\n    {{ out: 0 }}\n}}"
        ),
        "null",
        Some(TIMEOUT),
    );
    assert_eq!(error_kind(&env), Some("thread::timeout"), "got {env}");
    watcher.join().expect("the watcher");
    assert!(seen.load(Ordering::SeqCst), "the child marked 31338 ran before the timeout");
    assert!(
        wait_until_gone("31338", 10),
        "the child marked 31338 the timed-out external started should be reaped with it",
    );
}
