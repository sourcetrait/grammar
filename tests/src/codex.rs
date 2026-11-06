//! Codex's side of the Channel's Codex face, faked: its app-server and its `codex`.
//!
//! The app-server is a WebSocket server on a Unix socket in the test's directory,
//! speaking the frames the host's face sends and answers, and `codex` a script
//! answering `app-server daemon version` with that socket, so a host whose PATH
//! leads with its directory finds the fake as it would find Codex's daemon.

use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::{self, Message};

use crate::{Host, Packet, has_error_path, parse_line};

/// The name Codex's MCP client gives itself in `clientInfo`.
pub const CODEX_CLIENT: &str = "codex-mcp-client";

/// The name Claude Code's MCP client gives itself in `clientInfo`.
pub const CLAUDE_CLIENT: &str = "claude-code";

/// How the fake app-server answers each `turn/start`.
#[derive(Debug, Clone, Copy)]
pub enum TurnAnswers {
    /// Every turn started.
    All,
    /// The first `n` started, every later one refused with an error.
    First(usize),
}

/// A fake app-server: its socket, and every message its client sent, in order.
pub struct FakeAppServer {
    pub socket: PathBuf,
    rx: Receiver<Value>,
    _thread: JoinHandle<()>,
}

impl FakeAppServer {
    /// Listen on `<dir>/app-server.sock`, answering each connection's requests.
    ///
    /// Each message a client sends arrives on the receiver as its JSON, and a
    /// connection's end as `{ended: <how>}`, `close` for a close frame and `lost`
    /// for a read that failed.
    pub fn start(dir: &Path, turns: TurnAnswers) -> Self {
        let socket = dir.join("app-server.sock");
        let listener = UnixListener::bind(&socket).expect("bind the fake app-server");
        let (tx, rx) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else {
                    return;
                };
                let Ok(mut ws) = tungstenite::accept(stream) else {
                    continue;
                };
                let mut started = 0;
                loop {
                    let ended = match ws.read() {
                        Ok(Message::Text(text)) => {
                            let message: Value = serde_json::from_str(text.as_str()).unwrap_or(Value::Null);
                            if tx.send(message.clone()).is_err() {
                                return;
                            }
                            let Some(id) = message.get("id").cloned() else {
                                continue;
                            };
                            let turn = message["method"] == "turn/start";
                            if turn {
                                started += 1;
                            }
                            let refused = turn && matches!(turns, TurnAnswers::First(n) if started > n);
                            let reply = if refused {
                                json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32600, "message": "no turn for this test"}})
                            } else {
                                json!({"jsonrpc": "2.0", "id": id, "result": {}})
                            };
                            let _ = ws.send(Message::text(reply.to_string()));
                            continue;
                        }
                        Ok(Message::Close(_)) => "close",
                        Ok(_) => continue,
                        Err(_) => "lost",
                    };
                    let _ = tx.send(json!({"ended": ended}));
                    break;
                }
            }
        });
        Self {
            socket,
            rx,
            _thread: thread,
        }
    }

    /// The next message a client sent within `wait`, or none.
    pub fn next(&self, wait: Duration) -> Option<Value> {
        self.rx.recv_timeout(wait).ok()
    }

    /// The next message matching `pred` within `wait`, others passed by.
    pub fn await_message(&self, pred: impl Fn(&Value) -> bool, wait: Duration) -> Option<Value> {
        let deadline = Instant::now() + wait;
        loop {
            let left = deadline.checked_duration_since(Instant::now())?;
            let message = self.next(left)?;
            if pred(&message) {
                return Some(message);
            }
        }
    }

    /// The next `turn/start` within `wait`, its thread and the packet it carries.
    pub fn await_turn(&self, wait: Duration) -> Option<(String, Packet)> {
        let turn = self.await_message(|m| m["method"] == "turn/start", wait)?;
        let params = &turn["params"];
        assert_eq!(params["input"], json!([]), "a delivery's turn takes no input; got {turn}");
        let output = &params["toolOutput"];
        assert_eq!(output["namespace"], "grammar", "a delivery's namespace; got {turn}");
        assert_eq!(output["name"], "channel", "a delivery's name; got {turn}");
        let line = output["output"].as_str().expect("a delivery's output is its line");
        let packet = parse_line(line).unwrap_or_else(|| panic!("a delivery carries a packet; got {turn}"));
        let thread = params["threadId"].as_str().expect("a delivery names its thread").to_string();
        Some((thread, packet))
    }

    /// The next delivery whose packet matches `pred` within `wait`, others passed by.
    pub fn await_delivery(
        &self,
        pred: impl Fn(&Packet) -> bool,
        wait: Duration,
    ) -> Option<(String, Packet)> {
        let deadline = Instant::now() + wait;
        loop {
            let left = deadline.checked_duration_since(Instant::now())?;
            let delivery = self.await_turn(left)?;
            if pred(&delivery.1) {
                return Some(delivery);
            }
        }
    }
}

/// A `codex` in `<dir>/bin` whose daemon is at `socket`; that directory returned.
pub fn fake_codex(dir: &Path, socket: &Path) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("mkdir the fake codex's bin");
    let answer = json!({"socketPath": socket, "status": "running", "appServerVersion": "0.0.0"});
    let script = format!("#!/bin/sh\nprintf '%s\\n' '{answer}'\n");
    assert!(!socket.display().to_string().contains('\''), "the script quotes the socket whole");
    let path = bin.join("codex");
    std::fs::write(&path, script).expect("write the fake codex");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("the fake codex's mode");
    bin
}

/// `PATH` led by `bin`, so a host finds the fake `codex` there first.
pub fn path_with(bin: &Path) -> String {
    let rest = std::env::var("PATH").unwrap_or_default();
    format!("{}:{rest}", bin.display())
}

/// A Codex host's Channel through its face, the greeting delivered and verified.
pub struct CodexConsumer {
    pub host: Host,
    pub server: FakeAppServer,
    /// The Channel's inbox, as the open reported it.
    pub inbox: PathBuf,
    /// The host's nom, as its greeting carries it.
    pub nom: String,
}

impl CodexConsumer {
    /// The Codex handshake: `interface info` naming `thread`, the open, the verify.
    ///
    /// The greeting is delivered into the thread between the open and the verify.
    pub fn attach(mut host: Host, server: FakeAppServer, thread: &str) -> Self {
        let info = host.interface_with_meta("interface info", json!({"threadId": thread}));
        assert!(!has_error_path(&info), "`interface info` holds; got {info}");
        let open = host.interface("interface channel open async");
        assert!(!has_error_path(&open), "a Codex client's async open holds; got {open}");
        let result = &crate::envelope(&open)["result"];
        assert!(result["wss"].is_nothing(), "the Codex face has no endpoint; got {open}");
        let inbox = PathBuf::from(result["inbox"].as_str().expect("the open names its inbox"));
        let initialize = server
            .await_message(|m| m["method"] == "initialize", Duration::from_secs(10))
            .expect("the face initializes with the app-server");
        assert_eq!(initialize["params"]["clientInfo"]["name"], "grammar", "the face's name; got {initialize}");
        let (to, greeting) = server.await_turn(Duration::from_secs(10)).expect("the greeting is delivered");
        assert_eq!(to, thread, "the greeting goes to the thread info named");
        assert_eq!(greeting.model, "mcp/channel/Open", "the first delivery is the handshake; got {greeting:?}");
        let nom = nuval::NuVal::from_nuon(&greeting.event)
            .ok()
            .and_then(|event| event["mcp_nom"].as_str().map(str::to_string))
            .unwrap_or_else(|| panic!("the greeting carries the host's nom; got {greeting:?}"));
        let verified = host.interface("interface channel ready");
        assert!(!has_error_path(&verified), "the delivered greeting's ready verifies; got {verified}");
        Self { host, server, inbox, nom }
    }
}
