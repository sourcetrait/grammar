//! The remote engine commands end to end: an entry set or imported, opened, closed, and unset.
#![cfg(unix)]

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use sourcetrait_grammar_client::Endpoint;
use sourcetrait_grammar_tests::*;

/// How long a connect may take, its session's open and its worker's start among it.
const CONNECT: Duration = Duration::from_secs(90);

/// How long a worker may take to end once its session closes.
const END_WAIT: Duration = Duration::from_secs(30);

/// A definition answering the process it ran in.
const PID: &str = "def execute [data: nothing]: nothing -> record<pid: int> { {pid: $nu.pid} }";

/// A definition answering its worker's workdir and process.
const WHERE: &str = "def execute [data: nothing]: nothing -> record<work_dir: string, pid: int> { {work_dir: $env.EQUIP_WORK_DIR, pid: $nu.pid} }";

/// A host started in `root`'s agent directory, its remotes file not yet written.
fn bare_host(root: &Path) -> Host {
    let agent = root.join("agent");
    std::fs::create_dir_all(&agent).expect("the agent directory");
    Host::spawn_in(&root.join("scratch"), &[], &agent)
}

/// A response's result, its error envelope a panic.
fn answered(resp: &Value) -> NuVal {
    let envelope = extract_envelope(resp).unwrap_or_else(|| panic!("a result: {}", error_message(resp)));
    envelope["result"].clone()
}

/// An interface command on the MCP, a session's open within its wait.
fn interface_within(host: &mut Host, command: &str) -> Value {
    let id = host.request_interface(command);
    host.read_id_within(id, CONNECT)
}

/// A row's worker pid.
fn worker_pid(row: &NuVal) -> u32 {
    match row.get("worker_pid") {
        Some(NuVal::Int(pid)) => *pid as u32,
        other => panic!("a worker pid, not {other:?}"),
    }
}

/// A host key's OpenSSH line without its comment, as an entry pins it.
fn key_line(public: &str) -> String {
    public.split_whitespace().take(2).collect::<Vec<_>>().join(" ")
}

/// `set`'s flags for the worker's fields of `endpoint`: its command, id, and workdir.
fn worker_flags(endpoint: &Endpoint) -> String {
    format!(
        "--command {} --id {} --workdir {}",
        nu_literal(&endpoint.command),
        nu_literal(&endpoint.id),
        nu_literal(workdir(endpoint)),
    )
}

/// The workdir a test's endpoint names.
fn workdir(endpoint: &Endpoint) -> &str {
    endpoint.workdir.as_deref().expect("a test endpoint names its workdir")
}

/// The session `alias` names opened, a call run on it, then closed, its worker ending.
fn open_run_close(host: &mut Host, alias: &str) {
    let opened = answered(&interface_within(host, &format!("interface remote engine connect '{alias}'")));
    assert_eq!(opened["state"], "open", "{opened}");
    let pid = worker_pid(&opened);
    assert!(alive(pid), "the worker runs once connected");
    let listed = answered(&host.interface("interface remote engine list"));
    assert_eq!(listed[0]["state"], "open", "{listed}");
    let ran = host.call("execute", json!({"host": alias, "definition": PID, "data": "null"}));
    assert_eq!(answered(&ran)["pid"], i64::from(pid), "a call rides the open session: {ran}");

    let closed = answered(&host.interface(&format!("interface remote engine close '{alias}'")));
    assert!(closed.is_nothing(), "{closed}");
    assert!(eventually(END_WAIT, || !alive(pid)), "the close ends the worker");
    let after = answered(&host.interface(&format!("interface remote engine get '{alias}'")));
    assert_eq!(after["state"], "closed", "{after}");
    let again = host.interface(&format!("interface remote engine close '{alias}'"));
    assert_eq!(envelope_error_kind(&again).as_deref(), Some("remote::engine::not_open"), "{again}");
}

#[test]
fn an_entry_set_whole_opens_closes_and_is_unset() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let root = tempfile::tempdir().expect("a scratch root");
    let mut host = bare_host(root.path());
    let endpoint = sshd.endpoint("lab", &worker);

    assert_eq!(answered(&host.interface("interface remote engine list")), nuon("[]"));
    let partial = answered(&host.interface(&format!(
        "interface remote engine set 'lab' --address {} --user {}",
        nu_literal(&sshd.address()),
        nu_literal(&endpoint.user),
    )));
    assert_eq!(partial["missing"], nuon("[key, known_hosts]"), "{partial}");
    assert_eq!(partial["address"], sshd.address().as_str(), "{partial}");
    let refused = host.interface("interface remote engine connect 'lab'");
    assert_eq!(envelope_error_kind(&refused).as_deref(), Some("remote::engine::invalid_params"), "{refused}");
    let routed = host.call("execute", json!({"host": "lab", "definition": PID, "data": "null"}));
    assert_eq!(envelope_error_kind(&routed).as_deref(), Some("remote::engine::invalid_params"), "{routed}");

    let both = host.interface(&format!(
        "interface remote engine set 'lab' --known-hosts [{}] --known-hosts-file 'known_hosts'",
        nu_literal(&endpoint.host_keys[0]),
    ));
    assert_eq!(envelope_error_kind(&both).as_deref(), Some("remote::engine::invalid_params"), "{both}");
    let whole = answered(&host.interface(&format!(
        "interface remote engine set 'lab' --key {} --known-hosts [{}] {}",
        nu_literal(&endpoint.key),
        nu_literal(&endpoint.host_keys[0]),
        worker_flags(&endpoint),
    )));
    assert_eq!(whole["missing"], nuon("[]"), "{whole}");
    assert_eq!(whole["state"], "closed", "a set opens nothing: {whole}");
    let file = std::fs::read_to_string(root.path().join("agent/.grammar/remotes.toml")).expect("the remotes file");
    assert!(file.contains("[[engine]]"), "{file}");

    open_run_close(&mut host, "lab");

    let opened = answered(&interface_within(&mut host, "interface remote engine connect 'lab'"));
    let first = worker_pid(&opened);
    let moved = worker.root().join("moved");
    std::fs::create_dir_all(&moved).expect("a second workdir");
    let changed = answered(&host.interface(&format!(
        "interface remote engine set 'lab' --workdir {}",
        nu_literal(&moved.display().to_string()),
    )));
    assert_eq!(changed["workdir"], moved.display().to_string().as_str(), "{changed}");
    assert_eq!(changed["state"], "open", "a set leaves the open session standing: {changed}");
    let before = answered(&host.call("execute", json!({"host": "lab", "definition": WHERE, "data": "null"})));
    assert_eq!(before["pid"], i64::from(first), "the open session serves on: {before}");
    assert_eq!(before["work_dir"], workdir(&endpoint), "on the entry it opened on: {before}");
    let _ = answered(&host.interface("interface remote engine close 'lab'"));
    let id = host.request("execute", json!({"host": "lab", "definition": WHERE, "data": "null"}));
    let after = answered(&host.read_id_within(id, CONNECT));
    assert_eq!(after["work_dir"], moved.display().to_string().as_str(), "the next session takes the change: {after}");
    assert_ne!(after["pid"], i64::from(first), "a new worker: {after}");

    let live = answered(&host.interface("interface remote engine get 'lab'"));
    let second = worker_pid(&live);
    let unset = answered(&host.interface("interface remote engine unset 'lab'"));
    assert!(unset.is_nothing(), "{unset}");
    let orphan = answered(&host.interface("interface remote engine list"));
    assert_eq!(orphan[0]["alias"], "lab", "an unset leaves the live session listed: {orphan}");
    assert_eq!(orphan[0]["configured"], false, "{orphan}");
    assert_eq!(orphan[0]["state"], "open", "{orphan}");
    assert!(alive(second), "the worker lives on past the unset");
    let unknown = host.call("execute", json!({"host": "lab", "definition": PID, "data": "null"}));
    assert_eq!(envelope_error_kind(&unknown).as_deref(), Some("host::unknown"), "no call routes to it: {unknown}");
    let closed = answered(&host.interface("interface remote engine close 'lab'"));
    assert!(closed.is_nothing(), "{closed}");
    assert!(eventually(END_WAIT, || !alive(second)), "its close ends the worker");
    assert_eq!(answered(&host.interface("interface remote engine list")), nuon("[]"));
}

#[test]
fn an_import_takes_its_entry_from_an_ssh_config() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let root = tempfile::tempdir().expect("a scratch root");
    let mut host = bare_host(root.path());
    let endpoint = sshd.endpoint("lab", &worker);
    let known = root.path().join("known_hosts");
    let unknown = root.path().join("known_hosts_empty");
    std::fs::write(&known, format!("[{}]:{} {}\n", endpoint.host, endpoint.port, endpoint.host_keys[0])).expect("the known hosts");
    std::fs::write(&unknown, "").expect("an empty known hosts");
    let block = |name: &str, known: &Path| {
        format!(
            "Host {name}\n  HostName {}\n  Port {}\n  User {}\n  IdentityFile {}\n  UserKnownHostsFile {}\n  GlobalKnownHostsFile /dev/null\n",
            endpoint.host,
            endpoint.port,
            endpoint.user,
            endpoint.key,
            known.display(),
        )
    };
    let config = root.path().join("ssh_config");
    std::fs::write(&config, format!("{}{}", block("labhost", &known), block("stranger", &unknown))).expect("the ssh config");
    let config = nu_literal(&config.display().to_string());

    let imported = answered(&host.interface(&format!(
        "interface remote engine import 'lab' --from 'labhost' --ssh-config {config}"
    )));
    assert_eq!(imported["address"], sshd.address().as_str(), "{imported}");
    assert_eq!(imported["user"], endpoint.user.as_str(), "{imported}");
    assert_eq!(imported["key"], endpoint.key.as_str(), "{imported}");
    assert_eq!(imported["known_hosts"], "remote/lab/known_hosts", "{imported}");
    assert_eq!(imported["missing"], nuon("[]"), "the worker's fields take their defaults: {imported}");
    let grammar = root.path().join("agent/.grammar");
    let written = std::fs::read_to_string(grammar.join("remote/lab/known_hosts")).expect("the import's known hosts");
    let line = format!("[{}]:{} {}", endpoint.host, endpoint.port, key_line(&endpoint.host_keys[0]));
    assert!(written.starts_with("# grammar: written by `interface remote engine import`"), "{written}");
    assert!(written.lines().any(|l| l == line), "{written}");
    let again = answered(&host.interface(&format!(
        "interface remote engine import 'lab' --from 'labhost' --ssh-config {config}"
    )));
    assert_eq!(again["known_hosts"], "remote/lab/known_hosts", "an import rewrites its own file: {again}");

    let strange = host.interface(&format!("interface remote engine import 'other' --from 'stranger' --ssh-config {config}"));
    assert_eq!(envelope_error_kind(&strange).as_deref(), Some("remote::engine::invalid_params"), "{strange}");
    assert!(error_message(&strange).contains("no known host key"), "{strange}");
    let foreign = grammar.join("remote/ai/lab/known_hosts");
    std::fs::create_dir_all(foreign.parent().expect("its directory")).expect("a foreign file's directory");
    std::fs::write(&foreign, "# kept by hand\n").expect("a foreign known hosts file");
    let kept = host.interface(&format!("interface remote engine import 'ai/lab' --from 'labhost' --ssh-config {config}"));
    assert_eq!(envelope_error_kind(&kept).as_deref(), Some("remote::engine::invalid_params"), "{kept}");
    assert!(error_message(&kept).contains("isn't an import's"), "{kept}");
    assert_eq!(std::fs::read_to_string(&foreign).expect("the foreign file"), "# kept by hand\n", "left as it was");

    let whole = answered(&host.interface(&format!("interface remote engine set 'lab' {}", worker_flags(&endpoint))));
    assert_eq!(whole["missing"], nuon("[]"), "{whole}");
    open_run_close(&mut host, "lab");
}

#[test]
fn an_older_entry_is_refused_until_a_set_moves_it_to_the_new_fields() {
    let sshd = Sshd::start();
    let worker = TestWorker::new();
    let root = tempfile::tempdir().expect("a scratch root");
    let mut host = bare_host(root.path());
    let endpoint = sshd.endpoint("lab", &worker);
    let grammar = root.path().join("agent/.grammar");
    std::fs::create_dir_all(&grammar).expect("the config directory");
    let older = format!(
        "[[engine]]\nalias = \"lab\"\nhost = \"{}\"\nport = {}\nuser = \"{}\"\nkey = \"{}\"\nhost_keys = [\"{}\"]\n",
        endpoint.host, endpoint.port, endpoint.user, endpoint.key, endpoint.host_keys[0],
    );
    std::fs::write(grammar.join("remotes.toml"), older).expect("an older remotes file");
    let refused = host.interface("interface remote engine list");
    assert!(error_message(&refused).contains("`host` is retired; `address` takes its place"), "{refused}");

    let moved = answered(&host.interface(&format!(
        "interface remote engine set 'lab' --address {} --known-hosts [{}] {}",
        nu_literal(&sshd.address()),
        nu_literal(&endpoint.host_keys[0]),
        worker_flags(&endpoint),
    )));
    assert_eq!(moved["missing"], nuon("[]"), "{moved}");
    let file = std::fs::read_to_string(grammar.join("remotes.toml")).expect("the remotes file");
    for retired in ["host =", "port =", "host_keys ="] {
        assert!(!file.lines().any(|line| line.starts_with(retired)), "{retired} dropped: {file}");
    }
    open_run_close(&mut host, "lab");
}
