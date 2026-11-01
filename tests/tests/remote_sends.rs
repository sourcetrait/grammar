//! A remote eval's sends and asks reaching the MCP: its Channel, its config, its links.
#![cfg(unix)]

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use sourcetrait_grammar_client::Endpoint;
use sourcetrait_grammar_tests::*;

/// How long a first remote call may take, its session's open and its worker's start among it.
const FIRST_CALL: Duration = Duration::from_secs(90);

/// How long a packet may take to reach the Channel's reader.
const PACKET_WAIT: Duration = Duration::from_secs(15);

/// A definition sending one packet of `model` to the Channel.
fn send(model: &str) -> String {
    format!("def execute [data: nothing]: nothing -> record<event_id: string> {{ grimm channel send {model} {{n: 1}} }}")
}

/// `definition` executed on `host`, its first remote call's wait allowed.
fn execute_on(host: &mut Host, on: &str, definition: &str) -> Value {
    let id = host.request("execute", json!({"host": on, "definition": definition, "data": "null"}));
    host.read_id_within(id, FIRST_CALL)
}

/// A host started in `root`'s agent directory, its config `identity` then `engines`.
fn host_with(root: &Path, identity: Option<&GeneratedKeys>, engines: &[Endpoint]) -> Host {
    let agent = root.join("agent");
    let identity = identity
        .map(|keys| format!("[identity]\ncert = \"{}\"\nkey = \"{}\"\n\n", keys.public_key.display(), keys.private_key.display()))
        .unwrap_or_default();
    let engines: String = engines.iter().map(engine_toml).collect();
    write_source(&agent, ".grammar/config.toml", &format!("{identity}{engines}"));
    Host::spawn_in(&root.join("scratch"), &[], &agent)
}

#[test]
fn a_remote_send_before_the_channel_is_ready_is_refused() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let root = tempfile::tempdir().expect("a scratch root");
    let mut host = host_with(root.path(), None, &[sshd.endpoint("lab", &worker)]);
    let refused = execute_on(&mut host, "lab", &send("probe/Early"));
    let local = host.execute(&send("probe/Early"), "null");
    assert!(has_error_path(&refused), "a send before ready is refused on a worker: {refused}");
    assert_eq!(error_kinds(&refused), error_kinds(&local), "as it is locally: {refused} {local}");
}

#[test]
fn a_remote_evals_sends_and_asks_reach_the_mcp() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let root = tempfile::tempdir().expect("a scratch root");
    let keys = generate_keys("mcp", root.path());
    let host = host_with(root.path(), Some(&keys), &[sshd.endpoint("lab", &worker)]);
    let mut consumer = Consumer::attach(host, &keys.ca);

    let sent = execute_on(&mut consumer.host, "lab", &send("probe/Hello"));
    assert!(!has_error_path(&sent), "the send: {sent}");
    let packet = consumer
        .await_packet(|p| p.model == "probe/Hello", PACKET_WAIT)
        .expect("the worker's packet on the MCP's Channel");
    assert!(packet.from.starts_with("lab/thread/"), "its origin names the engine and the call: {packet:?}");
    assert_eq!(
        Some(packet.id.as_str()),
        envelope(&sent)["result"]["event_id"].as_str(),
        "the eval reads the id the Channel gave its packet",
    );

    let config = "def execute [data: nothing]: nothing -> record<cert: string> { {cert: (grimm get_config_all | get identity.cert)} }";
    let answered = consumer.host.call("execute", json!({"host": "lab", "definition": config, "data": "null"}));
    assert_eq!(
        envelope(&answered)["result"]["cert"].as_str(),
        Some(keys.public_key.display().to_string().as_str()),
        "the config is the MCP's: {answered}",
    );

    let far = "def execute [data: nothing]: nothing -> record<event_id: string> { grimm remote channel send 0123456789A probe/Far {n: 1} }";
    let remote = consumer.host.call("execute", json!({"host": "lab", "definition": far, "data": "null"}));
    let local = consumer.host.execute(far, "null");
    assert!(has_error_path(&remote), "no link answers that nom: {remote}");
    assert_eq!(error_kinds(&remote), error_kinds(&local), "the MCP's links answer a worker's send as a local one's: {remote} {local}");

    let flood = "def execute [data: nothing]: nothing -> nothing { for i in 1..60 { grimm channel send probe/Flood {i: $i} } }";
    let stopped = consumer.host.call("execute", json!({"host": "lab", "definition": flood, "data": "null"}));
    assert!(has_error_path(&stopped), "the flood is stopped: {stopped}");
    let spam = consumer
        .await_packet(|p| p.model == "mcp/channel/spam/Error", PACKET_WAIT)
        .expect("the stop's emergency");
    assert!(spam.event.contains("lab/thread/"), "the stop names the remote call: {spam:?}");
}

/// The first line of `id`'s emergency logs under `cache` holding `needle`, waited for.
#[cfg(feature = "test-hooks")]
fn wait_for_emergency(cache: &Path, id: &str, needle: &str, wait: Duration) -> Option<String> {
    let log_root = namespace_dir(cache, id, "default").join("log");
    let found = || -> Option<String> {
        let entries = std::fs::read_dir(&log_root).ok()?;
        entries.flatten().find_map(|entry| {
            let text = std::fs::read_to_string(entry.path().join("emergency.nuonl")).ok()?;
            text.lines().find(|line| line.contains(needle)).map(str::to_string)
        })
    };
    let deadline = std::time::Instant::now() + wait;
    while std::time::Instant::now() < deadline {
        if let Some(line) = found() {
            return Some(line);
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    found()
}

#[cfg(feature = "test-hooks")]
#[test]
fn a_workers_emergency_reaches_the_mcps_policy_under_its_host() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let root = tempfile::tempdir().expect("a scratch root");
    let agent = root.path().join("agent");
    write_source(&agent, ".grammar/config.toml", &engine_toml(&sshd.endpoint("lab", &worker)));
    let mut host = Host::spawn_in(&root.path().join("scratch"), &["--id", "probe"], &agent);
    let hang = "def execute [data: nothing]: nothing -> record<out: int> {\n__test_hang\n{ out: 0 }\n}";
    let id = host.request("execute", json!({"host": "lab", "definition": hang, "data": "null", "timeout": "500ms"}));
    let timed = host.read_id_within(id, FIRST_CALL);
    assert_eq!(envelope_error_kind(&timed).as_deref(), Some("thread::timeout"), "{timed}");
    let nonce = envelope(&timed)["nonce"].as_str().expect("its nonce").to_string();
    let line = wait_for_emergency(host.cache_home(), "probe", &nonce, Duration::from_secs(30))
        .unwrap_or_else(|| panic!("the worker's hung-thread emergency in the MCP's log for {nonce}"));
    assert!(line.contains("hung_engine_thread"), "{line}");
    assert!(line.contains("host: lab"), "the emergency names its host: {line}");
}
