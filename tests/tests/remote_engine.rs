//! The MCP's tools on a configured remote engine: its worker over SSH, its refusals, its end.
#![cfg(unix)]

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use sourcetrait_grammar_client::Endpoint;
use sourcetrait_grammar_tests::*;

/// How long a first remote call may take, its session's open and its worker's start among it.
const FIRST_CALL: Duration = Duration::from_secs(90);

/// How long a worker may take to end once the MCP does.
const END_WAIT: Duration = Duration::from_secs(30);

/// A definition answering where it ran: its workdir, its working directory, and its process.
const WHERE: &str = "def execute [data: nothing]: nothing -> record<work_dir: string, pwd: string, pid: int> { {work_dir: $env.EQUIP_WORK_DIR, pwd: $env.PWD, pid: $nu.pid} }";

/// A host started in an agent directory under `root` whose remotes file names `engines`.
fn host_with(root: &Path, engines: &[Endpoint]) -> Host {
    let agent = root.join("agent");
    write_source(&agent, ".grammar/remotes.toml", &engines.iter().map(engine_toml).collect::<String>());
    Host::spawn_in(&root.join("scratch"), &[], &agent)
}

/// Where `WHERE` ran by its response: the workdir, the working directory, and the process.
fn place(resp: &Value) -> (String, String, i64) {
    let envelope = extract_envelope(resp).unwrap_or_else(|| panic!("a result: {}", error_message(resp)));
    let result = &envelope["result"];
    let work_dir = result["work_dir"].as_str().expect("its work_dir").to_string();
    let pwd = result["pwd"].as_str().expect("its pwd").to_string();
    let pid = match result.get("pid") {
        Some(NuVal::Int(pid)) => *pid,
        other => panic!("a pid, not {other:?}"),
    };
    (work_dir, pwd, pid)
}

/// The envelope's nonce.
fn nonce_of(resp: &Value) -> String {
    envelope(resp)["nonce"].as_str().expect("its nonce").to_string()
}

#[test]
fn the_tools_run_on_a_configured_engines_worker_until_the_mcp_ends() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let root = tempfile::tempdir().expect("a scratch root");
    let mut host = host_with(root.path(), &[sshd.endpoint("lab", &worker)]);

    let id = host.request("execute", json!({"host": "lab", "definition": WHERE, "data": "null"}));
    let first = host.read_id_within(id, FIRST_CALL);
    let (work_dir, pwd, pid) = place(&first);
    assert_eq!(work_dir, worker.workdir().display().to_string(), "the worker's workdir");
    let entered = std::fs::canonicalize(worker.workdir()).expect("the workdir's real path");
    assert_eq!(pwd, entered.display().to_string(), "a remote eval starts in its worker's workdir");
    assert_ne!(pid as u32, host.pid(), "the worker's process, not the MCP's");
    let rerun = host.call("rerun", json!({"host": "lab", "nonce": nonce_of(&first), "data": "null"}));
    assert_eq!(place(&rerun), (work_dir.clone(), pwd.clone(), pid), "the rerun on the same worker");
    let (local_dir, _, local_pid) = place(&host.execute(WHERE, "null"));
    assert_ne!(local_dir, work_dir, "`local` stays on the MCP's engine");
    assert_eq!(local_pid as u32, host.pid());

    let agent = root.path().join("agent");
    let (made, rig) = host.equip_new("sourcetrait/probe", &agent);
    assert!(!has_error_path(&made), "equip new: {made}");
    write_source(&rig, "mod.nu", "export module m\n");
    write_source(&rig, "m/mod.nu", "export use which\n");
    write_source(&rig, "m/which/mod.nu", &valid_function_source("x: int", "from: string", "{ from: 'mcp' }"));
    let committed = host.commit("sourcetrait/probe");
    assert!(!has_error_path(&committed), "the commit: {committed}");
    let local = host.call_np("sourcetrait/probe:m:which", "{x: 1}");
    assert_eq!(envelope(&local)["result"]["from"].as_str(), Some("mcp"), "{local}");
    let remote = host.call("call", json!({"host": "lab", "namepath": "sourcetrait/probe:m:which", "data": "{x: 1}"}));
    let kind = envelope_error_kind(&remote).unwrap_or_default();
    assert!(kind.starts_with("equip::"), "the worker holds no rig the MCP committed: {remote}");

    let status = host.signal_and_wait(nix::sys::signal::Signal::SIGTERM, END_WAIT);
    assert!(status.is_some(), "the MCP ends on SIGTERM");
    assert!(eventually(END_WAIT, || !alive(pid as u32)), "the MCP's end ends its worker");
    assert!(worker.scratch_dirs().is_empty(), "the worker's scratch removed");
    assert!(sshd.log().contains("Accepted publickey"), "{}", sshd.log());
}

#[test]
fn a_remote_hosts_refusals_answer_by_kind() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let root = tempfile::tempdir().expect("a scratch root");
    let stray = TestKey::new(&root.path().join("stray_ed25519"));
    let mut stranger = sshd.endpoint("stranger", &worker);
    stranger.host_keys = vec![stray.public.clone()];
    let mut host = host_with(root.path(), &[stranger, sshd.endpoint("lab", &worker)]);

    let id = host.request("execute", json!({"host": "stranger", "definition": WHERE, "data": "null"}));
    let refused = host.read_id_within(id, FIRST_CALL);
    assert_eq!(envelope_error_kind(&refused).as_deref(), Some("remote::engine::host_key_rejected"), "{refused}");
    let unknown = host.call("execute", json!({"host": "elsewhere", "definition": WHERE, "data": "null"}));
    assert_eq!(envelope_error_kind(&unknown).as_deref(), Some("host::unknown"), "{unknown}");
    let unknown = host.call("interface", json!({"host": "elsewhere", "command": "interface info"}));
    assert_eq!(envelope_error_kind(&unknown).as_deref(), Some("host::unknown"), "{unknown}");
    let refused = host.call("interface", json!({"host": "stranger", "command": "interface process list"}));
    assert_eq!(envelope_error_kind(&refused).as_deref(), Some("remote::engine::host_key_rejected"), "{refused}");
    let channel = host.call("interface", json!({"host": "stranger", "command": "interface channel config"}));
    assert!(!has_error_path(&channel), "the Channel's commands answer on the MCP for any host: {channel}");
}

/// A definition that sleeps past any test's wait, until it is killed.
const SLEEP: &str = "def execute [data: nothing]: nothing -> nothing { sleep 30sec }";

/// The ids of a `purview list`'s purviews, each an `[id, patterns]` pair.
fn purview_ids(list: &NuVal) -> Vec<String> {
    list["result"]["purviews"]
        .as_list()
        .unwrap_or_default()
        .iter()
        .filter_map(|pair| pair.as_list().and_then(|pair| pair.first()).and_then(NuVal::as_str).map(str::to_string))
        .collect()
}

#[test]
fn the_interface_commands_run_on_the_engine_their_host_names() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let root = tempfile::tempdir().expect("a scratch root");
    let mut host = host_with(root.path(), &[sshd.endpoint("lab", &worker)]);

    let id = host.request("interface", json!({"host": "lab", "command": "interface info"}));
    let info = host.read_id_within(id, FIRST_CALL);
    let remote = extract_envelope(&info).unwrap_or_else(|| panic!("info: {}", error_message(&info)));
    assert_eq!(remote["result"]["work_dir"].as_str(), Some(worker.workdir().display().to_string().as_str()), "{info}");
    assert_eq!(remote["result"]["id"].as_str(), Some(WORKER_ID), "{info}");
    let local = envelope(&host.info());
    assert_ne!(remote["result"]["nom"].as_str(), local["result"]["nom"].as_str(), "the worker's nom, not the MCP's");

    let source = worker.workdir().join("equip").join("sourcetrait").join("probe");
    let made = host.call("interface", json!({"host": "lab", "command": format!("interface equip new '{}'", source.display())}));
    assert!(!has_error_path(&made), "equip new on the worker: {made}");
    let rig = source.join("rig");
    write_source(&rig, "mod.nu", "export module m\n");
    write_source(&rig, "m/mod.nu", "export use which\n");
    write_source(&rig, "m/which/mod.nu", &valid_function_source("x: int", "from: string", "{ from: 'worker' }"));
    let committed = host.call("interface", json!({"host": "lab", "command": "interface equip commit 'sourcetrait/probe'"}));
    assert!(!has_error_path(&committed), "the commit on the worker: {committed}");
    let called = host.call("call", json!({"host": "lab", "namepath": "sourcetrait/probe:m:which", "data": "{x: 1}"}));
    assert_eq!(envelope(&called)["result"]["from"].as_str(), Some("worker"), "{called}");
    let local = host.call_np("sourcetrait/probe:m:which", "{x: 1}");
    assert!(has_error_path(&local), "the MCP holds no rig the worker committed: {local}");

    let configured = host.call("interface", json!({"host": "lab", "command": "interface purview configure probe ['sourcetrait/probe:']"}));
    assert!(!has_error_path(&configured), "{configured}");
    let remote_list = envelope(&host.call("interface", json!({"host": "lab", "command": "interface purview list"})));
    let local_list = envelope(&host.interface("interface purview list"));
    assert!(purview_ids(&remote_list).contains(&"probe".to_string()), "{remote_list}");
    assert!(!purview_ids(&local_list).contains(&"probe".to_string()), "a host's purviews are its own: {local_list}");

    let call = host.request("execute", json!({"host": "lab", "definition": SLEEP, "data": "null"}));
    let mut listed = None;
    for _ in 0..100 {
        let rows = envelope(&host.call("interface", json!({"host": "lab", "command": "interface process list"})));
        if let Some(row) = rows["result"]["processes"].as_list().unwrap_or_default().first() {
            listed = row["nonce"].as_str().map(str::to_string);
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let listed = listed.expect("the call in flight on the worker");
    let local_rows = envelope(&host.interface("interface process list"));
    assert!(local_rows["result"]["processes"].as_list().unwrap_or_default().is_empty(), "nothing in flight on the MCP");
    let killed = host.call("interface", json!({"host": "lab", "command": format!("interface process kill '{listed}'")}));
    assert!(!has_error_path(&killed), "{killed}");
    let ended = host.read_id_within(call, END_WAIT);
    assert!(has_error_path(&ended), "the killed call answers its failure: {ended}");
}
