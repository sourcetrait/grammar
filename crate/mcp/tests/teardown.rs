//! External teardown: a timed-out eval's external child is reaped (the timeout ->
//! cancel -> tree-kill path), and so is that child's own child. In-process:
//! `nu_timeout` drives the real dispatch, and the reap is asserted by scanning the
//! processes' command lines, from /proc on Linux and from `ps` elsewhere.

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
#[cfg(not(target_os = "linux"))]
fn any_cmdline_contains(marker: &str) -> bool {
    let out = std::process::Command::new("ps")
        .args(["-A", "-o", "args="])
        .output()
        .expect("run ps");
    String::from_utf8_lossy(&out.stdout).lines().any(|line| line.contains(marker))
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
    // The eval blocks on `^sleep 31337`; the 500ms timeout triggers cancel + a
    // tree-kill of the tracked external, which also unblocks the waiting thread.
    let env = s.execute_timeout(
        "def execute [data: nothing]: nothing -> record<out: int> {\n    ^sleep 31337\n    { out: 0 }\n}",
        "null",
        Some("500ms"),
    );
    assert_eq!(error_kind(&env), Some("thread::timeout"), "got {env}");
    assert!(
        wait_until_gone("31337", 10),
        "the timed-out `sleep 31337` external should be reaped by the tree-kill",
    );
}

#[test]
fn timed_out_external_tree_is_reaped() {
    let s = TestServer::new();
    // The tracked external is `xargs`; the `sleep 31338` it starts is its own child,
    // which only the tree-kill's walk of the process table reaches.
    let env = s.execute_timeout(
        "def execute [data: nothing]: nothing -> record<out: int> {\n    '31338' | ^xargs sleep\n    { out: 0 }\n}",
        "null",
        Some("500ms"),
    );
    assert_eq!(error_kind(&env), Some("thread::timeout"), "got {env}");
    assert!(
        wait_until_gone("31338", 10),
        "the `sleep 31338` the timed-out `xargs` started should be reaped with it",
    );
}
