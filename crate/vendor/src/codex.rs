//! Codex: its client's name, its thread's `_meta` key, and its app-server's daemon.
use crate::*;

#[cfg(unix)]
pub(crate) mod client;

/// The name Codex's MCP client gives itself in `clientInfo`.
pub const CODEX_CLIENT: &str = "codex-mcp-client";

/// The `_meta` key a Codex tool call names its calling thread under.
pub const THREAD_ID_META: &str = "threadId";

/// How long `codex app-server daemon version` may take before it is killed.
const DAEMON_VERSION_WAIT: Duration = Duration::from_secs(10);

/// What `codex app-server daemon version` answers of the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonStatus {
    /// The app-server's control socket.
    pub socket: PathBuf,
    /// The daemon's own word for its state.
    pub status: String,
    /// The app-server's version, where the daemon names one.
    pub app_server_version: Option<String>,
}

/// The daemon's answer as Codex writes it, read at the edge alone.
#[derive(ser::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DaemonAnswer {
    socket_path: String,
    status: String,
    app_server_version: Option<String>,
}

/// The daemon's JSON answer, its socket required and not blank.
pub fn parse_daemon_status(json: &str) -> Result<DaemonStatus, String> {
    let answer: DaemonAnswer = serde_json::from_str(json)
        .map_err(|e| format!("codex app-server daemon version answered no status: {e}"))?;
    if answer.socket_path.trim().is_empty() {
        return Err("codex app-server daemon version named no socket".to_string());
    }
    Ok(DaemonStatus {
        socket: PathBuf::from(answer.socket_path),
        status: answer.status,
        app_server_version: answer.app_server_version,
    })
}

/// `codex app-server daemon version`, the one Codex command run here, parsed.
pub fn daemon_status() -> Result<DaemonStatus, String> {
    let what = "codex app-server daemon version";
    let mut child = std::process::Command::new("codex")
        .args(["app-server", "daemon", "version"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("{what}: {e}"))?;
    let deadline = Instant::now() + DAEMON_VERSION_WAIT;
    loop {
        match child.try_wait().map_err(|e| format!("{what}: {e}"))? {
            Some(_) => break,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{what} took past {} s", DAEMON_VERSION_WAIT.as_secs()));
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    let out = child.wait_with_output().map_err(|e| format!("{what}: {e}"))?;
    if !out.status.success() {
        return Err(format!("{what} failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    parse_daemon_status(&String::from_utf8_lossy(&out.stdout))
}
