//! The Channel's face chosen by the client's `clientInfo`, and the Codex face.
//!
//! A Claude Code client opens the wss hub and a client the host doesn't know a
//! sync Channel; a Codex client opens the Codex face, which reaches a fake
//! app-server through a fake `codex`'s daemon answer and delivers each packet into
//! the thread `interface info` named.
#![cfg(unix)]

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_tests::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// How long a test waits on a delivery or a notice.
const WAIT: Duration = Duration::from_secs(15);

/// A host whose client is `client`, its identity under `root`, so a wss open holds.
fn host_with_identity(root: &Path, client: &str) -> Host {
    let keys = generate_keys("grammar", &root.join("channel"));
    write_host_config(&root.join("host"), &keys, "");
    Host::spawn_as(
        &root.join("scratch"),
        client,
        &["--id", "face", "--namespace", "default"],
        &[],
        Some(&root.join("host")),
    )
}

/// A Codex client's host, its PATH finding the fake `codex` in `bin` first.
fn codex_host(root: &Path, bin: &Path, cwd: Option<&Path>) -> Host {
    let path = path_with(bin);
    Host::spawn_as(
        &root.join("scratch"),
        CODEX_CLIENT,
        &["--id", "codex", "--namespace", "default"],
        &[("PATH", path.as_str())],
        cwd,
    )
}

/// The stream an open's mode named, or none.
fn opened_uri(open: &Value) -> Option<String> {
    envelope(open)["result"]["mode"]["uri"].as_str().map(str::to_string)
}

/// A `grmr channel send` of `model` from a definition, its event id.
fn channel_send(host: &mut Host, model: &str, msg: &str) -> Result<String, Value> {
    let definition = format!(
        "def execute [data: record<msg: string>]: nothing -> record<event_id: string> {{\n    let sent = (grmr channel send '{model}' {{msg: $data.msg}})\n    {{event_id: $sent.event_id}}\n}}",
    );
    let resp = host.execute(&definition, &json!({"msg": msg}).to_string());
    if has_error_path(&resp) {
        return Err(resp);
    }
    Ok(envelope(&resp)["result"]["event_id"].as_str().expect("a send's event id").to_string())
}

#[tested]
fn a_claude_client_opens_the_wss_hub() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = host_with_identity(t.temp_dir(), CLAUDE_CLIENT);
    let open = host.interface("interface channel open");
    assert!(!has_error_path(&open), "Claude Code's open holds; got {open}");
    assert_eq!(envelope(&open)["result"]["mode"]["kind"], "async", "Claude Code's mode; got {open}");
    let uri = opened_uri(&open).expect("Claude Code's open names its stream");
    assert!(uri.starts_with("wss://127.0.0.1:"), "the hub's stream; got {uri}");
}

#[tested]
fn any_other_client_opens_a_sync_channel() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = host_with_identity(t.temp_dir(), "an-unknown-client");
    let open = host.interface("interface channel open");
    assert!(!has_error_path(&open), "an unknown client's open holds; got {open}");
    assert_eq!(envelope(&open)["result"]["mode"]["kind"], "sync", "an unknown client's mode; got {open}");
    assert!(opened_uri(&open).is_none(), "a sync Channel names no stream; got {open}");
}

#[tested]
fn a_codex_open_names_interface_info_until_a_thread_is_kept() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let bin = fake_codex(root, &root.join("none.sock"));
    let mut host = codex_host(root, &bin, None);
    let open = host.interface("interface channel open");
    assert_eq!(envelope_error_kind(&open).as_deref(), Some("channel::start"), "the refusal; got {open}");
    assert!(error_message(&open).contains("interface info"), "the refusal names `interface info`; got {open}");
    let sync = host.interface("interface channel sync");
    assert_eq!(envelope_error_kind(&sync).as_deref(), Some("channel::not_open"), "the refusal opened nothing; got {sync}");
}

#[tested]
fn a_codex_open_with_no_daemon_names_the_socket_it_looked_for() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let socket = root.join("none.sock");
    let bin = fake_codex(root, &socket);
    let mut host = codex_host(root, &bin, None);
    let info = host.interface_with_meta("interface info", json!({"threadId": "thread-a"}));
    assert!(!has_error_path(&info), "`interface info` holds; got {info}");
    let open = host.interface("interface channel open");
    assert_eq!(envelope_error_kind(&open).as_deref(), Some("channel::start"), "the refusal; got {open}");
    let message = error_message(&open);
    assert!(message.contains(&socket.display().to_string()), "the refusal names the socket; got {message}");
    let again = host.interface("interface channel open");
    assert_eq!(
        envelope_error_kind(&again).as_deref(),
        Some("channel::start"),
        "a failed start leaves the Channel closed, so the next open tries again; got {again}",
    );
}

#[tested]
fn a_codex_channel_delivers_each_packet_into_the_thread_info_named() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let sockets = tempfile::tempdir().expect("a short directory for the socket");
    let server = FakeAppServer::start(sockets.path(), TurnAnswers::All);
    let bin = fake_codex(root, &server.socket);
    let host = codex_host(root, &bin, None);
    let mut c = CodexConsumer::attach(host, server, "thread-a");

    let sent = channel_send(&mut c.host, "test/Ping", "hello-codex").expect("the send holds");
    let (to, packet) = c
        .server
        .await_delivery(|p| p.model == "test/Ping", WAIT)
        .expect("the send is delivered");
    assert_eq!(to, "thread-a", "a delivery goes to the thread info named");
    assert_eq!(packet.id, sent, "the delivered packet is the one sent");
    assert!(packet.event.contains("hello-codex"), "its event rides whole; got {packet:?}");

    let refreshed = c.host.interface_with_meta("interface info", json!({"threadId": "thread-b"}));
    assert!(!has_error_path(&refreshed), "`interface info` holds; got {refreshed}");
    channel_send(&mut c.host, "test/Pong", "hello-again").expect("the send holds");
    let (to, _) = c
        .server
        .await_delivery(|p| p.model == "test/Pong", WAIT)
        .expect("the next send is delivered");
    assert_eq!(to, "thread-b", "a later info moves the deliveries to its thread");

    let closed = c.host.interface("interface channel close");
    assert!(!has_error_path(&closed), "the close holds; got {closed}");
    let ended = c
        .server
        .await_message(|m| m.get("ended").is_some(), WAIT)
        .expect("the face ends its session at the close");
    assert_eq!(ended["ended"], "close", "the face sends its close; got {ended}");
}

#[tested]
fn a_refused_delivery_ends_the_codex_faces_peer() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let sockets = tempfile::tempdir().expect("a short directory for the socket");
    let server = FakeAppServer::start(sockets.path(), TurnAnswers::First(1));
    let bin = fake_codex(root, &server.socket);
    let host = codex_host(root, &bin, None);
    let mut c = CodexConsumer::attach(host, server, "thread-a");

    channel_send(&mut c.host, "test/Ping", "refused").expect("the send itself holds");
    let deadline = Instant::now() + WAIT;
    loop {
        let open = c.host.interface("interface channel open");
        if envelope_error_kind(&open).as_deref() == Some("channel::peer_gone") {
            break;
        }
        assert!(Instant::now() < deadline, "the refused delivery ends the peer; last open {open}");
        std::thread::sleep(Duration::from_millis(100));
    }
    let after = loop {
        match channel_send(&mut c.host, "test/Ping", "after") {
            Err(refused) => break refused,
            Ok(_) => {
                assert!(Instant::now() < deadline, "a send after the peer's end is refused");
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    };
    assert!(error_message(&after).contains("hub is gone"), "a send after it answers HubGone; got {after}");
}

#[tested]
fn a_relay_is_delivered_into_the_codex_thread() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let cert_a = generate_keys("a", &root.join("cert_a"));
    let cert_b = generate_keys("b", &root.join("cert_b"));
    let listen = format!("127.0.0.1:{}", free_port());

    let sockets = tempfile::tempdir().expect("a short directory for the socket");
    let server = FakeAppServer::start(sockets.path(), TurnAnswers::All);
    let bin = fake_codex(root, &server.socket);
    let b_dir = root.join("host_b");
    write_host_config(&b_dir, &cert_b, &remote_listener(&listen, &cert_a.public_key));
    let host_b = codex_host(&root.join("b"), &bin, Some(&b_dir));
    let mut b = CodexConsumer::attach(host_b, server, "thread-b");

    let a_dir = root.join("host_a");
    write_host_config(&a_dir, &cert_a, &remote_connector(&listen, &cert_b.public_key));
    let host_a = Host::spawn_claude_in(&root.join("scratch_a"), &["--id", "link_a", "--namespace", "default"], &a_dir);
    let mut a = Consumer::attach(host_a, &cert_a.ca);

    let listening = b.host.interface("interface remote channel listen 'peer'");
    assert!(!has_error_path(&listening), "B listens; got {listening}");
    let connected = a.host.interface("interface remote channel connect 'peer'");
    assert!(!has_error_path(&connected), "A connects; got {connected}");
    b.server
        .await_delivery(|p| p.model == "remote/channel/Connected" && p.event.contains(&a.nom), WAIT)
        .expect("B's Connected is delivered into its thread");

    let definition = "def execute [data: record<nom: string>]: nothing -> record<id: string> {\n    let sent = (grmr remote channel send $data.nom 'test/Ping' {note: 'relayed'})\n    {id: $sent.event_id}\n}";
    let sent = a.host.execute(definition, &json!({"nom": b.nom}).to_string());
    assert!(!has_error_path(&sent), "A's remote send holds; got {sent}");
    let (to, packet) = b
        .server
        .await_delivery(|p| p.model == "test/Ping", WAIT)
        .expect("the relay is delivered into B's thread");
    assert_eq!(to, "thread-b", "the relay goes to the thread B's info named");
    assert_eq!(packet.from, format!("remote/channel/{}", a.nom), "the relay is from A; got {packet:?}");
    assert!(packet.event.contains("relayed"), "its event rides whole; got {packet:?}");
}
