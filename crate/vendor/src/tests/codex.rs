//! Codex's daemon answer read at the edge, and the app-server client on a socket.
use crate::*;

#[test]
fn the_daemons_answer_names_its_socket_or_is_refused() {
    let status = parse_daemon_status(
        r#"{"socketPath": "/tmp/codex-daemon-1000/app-server-control.sock", "status": "running", "appServerVersion": "0.166.0"}"#,
    )
    .expect("an answer");
    assert_eq!(status.socket, PathBuf::from("/tmp/codex-daemon-1000/app-server-control.sock"));
    assert_eq!(status.status, "running");
    assert_eq!(status.app_server_version.as_deref(), Some("0.166.0"));
    for bad in [r#"{"status": "running"}"#, r#"{"socketPath": " ", "status": "running"}"#, "not json"] {
        assert!(parse_daemon_status(bad).is_err(), "{bad} is refused");
    }
}

#[cfg(unix)]
mod client {
    use crate::*;
    use serde_json::{Value, json};
    use std::os::unix::net::UnixListener;
    use std::thread::JoinHandle;
    use tungstenite::Message;

    /// The tool output each case delivers.
    const OUTPUT: ToolOutput<'static> = ToolOutput {
        name: "channel",
        namespace: "grammar",
        output: "{id: abc}",
    };

    /// A test app-server on a Unix socket in `dir`, each request answered by `answer`.
    ///
    /// A request `answer` gives nothing for goes unanswered, and the server's thread
    /// hands back every message it read, in order.
    fn serve(
        dir: &Path,
        answer: fn(&Value) -> Option<Value>,
    ) -> (PathBuf, JoinHandle<Vec<Value>>) {
        let socket = dir.join("app-server.sock");
        let listener = UnixListener::bind(&socket).expect("a socket");
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("a client");
            let mut ws = tungstenite::accept(stream).expect("the handshake");
            let mut read = Vec::new();
            while let Ok(message) = ws.read() {
                let Message::Text(text) = message else {
                    continue;
                };
                let request: Value = serde_json::from_str(text.as_str()).expect("a request");
                if request.get("id").is_some() && let Some(reply) = answer(&request) {
                    let _ = ws.send(Message::text(reply.to_string()));
                }
                read.push(request);
            }
            read
        });
        (socket, handle)
    }

    /// Every request answered with an empty result.
    fn result(request: &Value) -> Option<Value> {
        Some(json!({"jsonrpc": "2.0", "id": request["id"], "result": {}}))
    }

    #[test]
    fn a_client_initializes_and_starts_a_turn_with_the_tool_output() {
        let dir = tempfile::tempdir().expect("a directory");
        let (socket, server) = serve(dir.path(), result);
        let mut client = AppServerClient::connect(&socket, "grammar", "0.0.0-0", Duration::from_secs(5)).expect("a session");
        client.turn_start("thread-1", &OUTPUT).expect("the turn starts");
        client.close();
        let read = server.join().expect("the server");
        let methods: Vec<&str> = read.iter().map(|r| r["method"].as_str().unwrap_or_default()).collect();
        assert_eq!(methods, ["initialize", "initialized", "turn/start"]);
        assert_eq!(read[0]["params"]["clientInfo"]["name"], "grammar");
        assert_eq!(read[2]["params"]["threadId"], "thread-1");
        assert_eq!(read[2]["params"]["input"], json!([]));
        assert_eq!(read[2]["params"]["toolOutput"], json!({"name": "channel", "namespace": "grammar", "output": "{id: abc}"}));
    }

    #[test]
    fn an_error_answer_is_the_requests_refusal() {
        let dir = tempfile::tempdir().expect("a directory");
        let refuse_turns = |request: &Value| match request["method"].as_str() {
            Some("turn/start") => Some(json!({"jsonrpc": "2.0", "id": request["id"], "error": {"code": -32600, "message": "no such thread"}})),
            _ => result(request),
        };
        let (socket, _server) = serve(dir.path(), refuse_turns);
        let mut client = AppServerClient::connect(&socket, "grammar", "0.0.0-0", Duration::from_secs(5)).expect("a session");
        let refused = client.turn_start("thread-1", &OUTPUT).expect_err("an error answer");
        assert!(refused.contains("turn/start refused") && refused.contains("no such thread"), "{refused}");
    }

    #[test]
    fn an_unanswered_request_times_out_by_its_bound() {
        let dir = tempfile::tempdir().expect("a directory");
        let ignore_turns = |request: &Value| match request["method"].as_str() {
            Some("turn/start") => None,
            _ => result(request),
        };
        let (socket, _server) = serve(dir.path(), ignore_turns);
        let mut client = AppServerClient::connect(&socket, "grammar", "0.0.0-0", Duration::from_millis(300)).expect("a session");
        let started = Instant::now();
        let late = client.turn_start("thread-1", &OUTPUT).expect_err("no answer");
        assert!(late.contains("no answer to turn/start within 300 ms"), "{late}");
        assert!(started.elapsed() < Duration::from_secs(3), "the bound holds: {:?}", started.elapsed());
    }

    #[test]
    fn a_missing_socket_is_refused_by_its_path() {
        let dir = tempfile::tempdir().expect("a directory");
        let socket = dir.path().join("none.sock");
        let refused = AppServerClient::connect(&socket, "grammar", "0.0.0-0", Duration::from_secs(1))
            .err()
            .expect("no socket");
        assert!(refused.contains(&socket.display().to_string()), "{refused}");
    }
}
