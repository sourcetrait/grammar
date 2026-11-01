//! External teardown: a timed-out eval's external child is reaped (the timeout ->
//! cancel -> tree-kill path), and so is that child's own child. In-process:
//! `nu_timeout` drives the real dispatch, and the reap is asserted by scanning the
//! processes' command lines: from /proc on Linux, from `ps` on the other Unixes, and
//! from PowerShell's process query on Windows.

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

/// Any command line Windows' process table holds containing `marker`, read by
/// PowerShell's CIM query, Windows having no `ps`.
#[cfg(windows)]
fn any_cmdline_contains(marker: &str) -> bool {
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_Process | ForEach-Object { $_.CommandLine }",
        ])
        .output()
        .expect("run powershell");
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
    // The eval blocks on an external that runs for hours, `sleep 31337` on Unix and,
    // Windows having no sleep, a `ping` of 31337 echoes there; the 500ms timeout
    // triggers cancel + a tree-kill of the tracked external, which also unblocks the
    // waiting thread.
    let external = if cfg!(windows) { "^ping -n 31337 127.0.0.1" } else { "^sleep 31337" };
    let env = s.execute_timeout(
        &format!(
            "def execute [data: nothing]: nothing -> record<out: int> {{\n    {external}\n    {{ out: 0 }}\n}}"
        ),
        "null",
        Some("500ms"),
    );
    assert_eq!(error_kind(&env), Some("thread::timeout"), "got {env}");
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
    // `sleep 31338` on Unix, `cmd` starting a `ping` of 31338 echoes on Windows.
    let external = if cfg!(windows) {
        "^cmd /c ping -n 31338 127.0.0.1"
    } else {
        "'31338' | ^xargs sleep"
    };
    let env = s.execute_timeout(
        &format!(
            "def execute [data: nothing]: nothing -> record<out: int> {{\n    {external}\n    {{ out: 0 }}\n}}"
        ),
        "null",
        Some("500ms"),
    );
    assert_eq!(error_kind(&env), Some("thread::timeout"), "got {env}");
    assert!(
        wait_until_gone("31338", 10),
        "the child marked 31338 the timed-out external started should be reaped with it",
    );
}
