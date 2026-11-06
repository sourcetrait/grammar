//! Codex's app-server: a WebSocket on its control socket, JSON-RPC at the edge.
use crate::*;
use serde_json::{Value, json};
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use tungstenite::{Message, WebSocket, client::IntoClientRequest};

/// A tool's output a delivery carries: its name, its namespace, and its text.
#[derive(Debug, Clone, Copy)]
pub struct ToolOutput<'a> {
    pub name: &'a str,
    pub namespace: &'a str,
    pub output: &'a str,
}

/// A Unix stream whose every read returns by its deadline.
struct DeadlineStream {
    stream: UnixStream,
    deadline: Instant,
}

impl Read for DeadlineStream {
    fn read(
        &mut self,
        buf: &mut [u8],
    ) -> io::Result<usize> {
        let left = self.deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        self.stream.set_read_timeout(Some(left))?;
        self.stream.read(buf)
    }
}

impl Write for DeadlineStream {
    fn write(
        &mut self,
        buf: &[u8],
    ) -> io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

/// Whether a read failed by its deadline rather than by the stream.
fn timed_out(err: &tungstenite::Error) -> bool {
    match err {
        tungstenite::Error::Io(err) => matches!(err.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut),
        _ => false,
    }
}

/// One session with an app-server: its socket's WebSocket, its next id, its bound.
pub struct AppServerClient {
    ws: WebSocket<DeadlineStream>,
    next_id: u64,
    wait: Duration,
}

impl AppServerClient {
    /// The socket's session opened and `initialize` answered, each step within `wait`.
    pub fn connect(
        socket: &Path,
        name: &str,
        version: &str,
        wait: Duration,
    ) -> Result<Self, String> {
        let stream = UnixStream::connect(socket).map_err(|e| format!("connect {}: {e}", socket.display()))?;
        let stream = DeadlineStream {
            stream,
            deadline: Instant::now() + wait,
        };
        let request = "ws://localhost/".into_client_request().map_err(|e| e.to_string())?;
        let (ws, _) = tungstenite::client(request, stream).map_err(|e| format!("the app-server's handshake: {e}"))?;
        let mut client = Self { ws, next_id: 1, wait };
        let info = json!({"clientInfo": {"name": name, "version": version}, "capabilities": {"experimentalApi": true}});
        client.request("initialize", info)?;
        client.notify("initialized")?;
        Ok(client)
    }

    /// `turn/start` on `thread`, its input empty and `output` a tool's.
    pub fn turn_start(
        &mut self,
        thread: &str,
        output: &ToolOutput<'_>,
    ) -> Result<(), String> {
        let tool = json!({"name": output.name, "namespace": output.namespace, "output": output.output});
        self.request("turn/start", json!({"threadId": thread, "input": [], "toolOutput": tool}))
            .map(|_| ())
    }

    /// The session closed, best effort: the app-server sends no close of its own back.
    pub fn close(mut self) {
        let _ = self.ws.close(None);
        let _ = self.ws.flush();
    }

    /// A notification, which takes no answer.
    fn notify(
        &mut self,
        method: &str,
    ) -> Result<(), String> {
        let text = json!({"jsonrpc": "2.0", "method": method}).to_string();
        self.ws.send(Message::text(text)).map_err(|e| format!("send {method}: {e}"))
    }

    /// One request and its answer by its id within the bound, an `error` its refusal.
    fn request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let text = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string();
        self.ws.send(Message::text(text)).map_err(|e| format!("send {method}: {e}"))?;
        let deadline = Instant::now() + self.wait;
        self.ws.get_mut().deadline = deadline;
        let late = || format!("no answer to {method} within {} ms", self.wait.as_millis());
        loop {
            let message = match self.ws.read() {
                Ok(message) => message,
                Err(err) if timed_out(&err) => return Err(late()),
                Err(err) => return Err(format!("{method}: {err}")),
            };
            if Instant::now() >= deadline {
                return Err(late());
            }
            let text = match message {
                Message::Text(text) => text,
                Message::Close(frame) => return Err(format!("the app-server closed during {method}: {frame:?}")),
                _ => continue,
            };
            let got: Value = serde_json::from_str(text.as_str()).map_err(|e| format!("{method}'s answer: {e}"))?;
            if got.get("id") != Some(&json!(id)) || got.get("method").is_some() {
                continue;
            }
            if let Some(error) = got.get("error") {
                return Err(format!("{method} refused: {error}"));
            }
            return Ok(got.get("result").cloned().unwrap_or(Value::Null));
        }
    }
}
