//! A remote engine reached through another engine's host: its session over the jump's.
#![cfg(unix)]

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use sourcetrait_grammar_client::{Endpoint, Jump};
use sourcetrait_grammar_tests::*;

/// How long a first remote call may take, both hops' logins and grammar-ssh's start among it.
const FIRST_CALL: Duration = Duration::from_secs(90);

/// How long grammar-ssh or a session may take to end once its session closes.
const END_WAIT: Duration = Duration::from_secs(30);

/// A definition answering the process it ran in.
const PID: &str = "def execute [data: nothing]: nothing -> record<pid: int> { {pid: $nu.pid} }";

/// A host whose remotes file names `engines`, started in an agent directory under `root`.
fn host_with(root: &Path, engines: &[Endpoint]) -> Host {
    let agent = root.join("agent");
    write_source(&agent, ".grammar/remotes.toml", &engines.iter().map(engine_toml).collect::<String>());
    Host::spawn_in(&root.join("scratch"), &[], &agent)
}

/// `target` reached through `gate`'s host.
fn through(target: Endpoint, gate: &Endpoint) -> Endpoint {
    Endpoint {
        jump: Some(Jump::of(gate)),
        ..target
    }
}

/// A response's pid, its error envelope a panic.
fn pid_of(resp: &Value) -> i64 {
    let envelope = extract_envelope(resp).unwrap_or_else(|| panic!("a result: {}", error_message(resp)));
    match envelope["result"].get("pid") {
        Some(NuVal::Int(pid)) => *pid,
        other => panic!("a pid, not {other:?}"),
    }
}

#[test]
fn an_engine_opens_through_its_jump_and_closes_with_it() {
    let gate_sshd = Sshd::start_jump();
    let lab_sshd = Sshd::start();
    let account = TestAccount::new();
    let root = tempfile::tempdir().expect("a scratch root");
    let gate = gate_sshd.endpoint("gate", &account);
    let lab = through(lab_sshd.endpoint("lab", &account), &gate);
    let direct = lab_sshd.endpoint("direct", &account);
    let mut host = host_with(root.path(), &[gate, lab, direct]);
    let closes = |sshd: &Sshd| sshd.log().matches("the MCP closed the session").count();

    let id = host.request("execute", json!({"host": "direct", "definition": PID, "data": "null"}));
    let direct_pid = pid_of(&host.read_id_within(id, FIRST_CALL));
    let closed = host.interface("interface remote engine close 'direct'");
    assert!(!has_error_path(&closed), "{closed}");
    assert!(eventually(END_WAIT, || !alive(direct_pid as u32)), "the direct close ends its grammar-ssh");
    assert!(eventually(END_WAIT, || closes(&lab_sshd) == 1), "a direct close reaches its host: {}", lab_sshd.log());

    let id = host.request("execute", json!({"host": "lab", "definition": PID, "data": "null"}));
    let pid = pid_of(&host.read_id_within(id, FIRST_CALL));
    assert_ne!(pid as u32, host.pid(), "grammar-ssh's process, not the MCP's");
    assert!(gate_sshd.log().contains("Accepted publickey"), "the jump's login: {}", gate_sshd.log());
    assert!(lab_sshd.log().contains("Accepted publickey"), "the engine's own login: {}", lab_sshd.log());
    let again = host.call("execute", json!({"host": "lab", "definition": PID, "data": "null"}));
    assert_eq!(pid_of(&again), pid, "the next call rides the same session");

    let closed = host.interface("interface remote engine close 'lab'");
    assert!(!has_error_path(&closed), "{closed}");
    assert!(eventually(END_WAIT, || !alive(pid as u32)), "the close ends grammar-ssh");
    assert!(eventually(END_WAIT, || closes(&lab_sshd) == 2), "the engine's close reaches it: {}", lab_sshd.log());
    assert!(eventually(END_WAIT, || closes(&gate_sshd) == 1), "the jump's ends with it: {}", gate_sshd.log());
}

#[test]
fn a_set_or_an_unset_breaking_a_jump_is_refused() {
    let gate_sshd = Sshd::start_jump();
    let account = TestAccount::new();
    let root = tempfile::tempdir().expect("a scratch root");
    let mut host = host_with(root.path(), &[gate_sshd.endpoint("gate", &account)]);
    let refused = |resp: &Value, words: &str| {
        assert_eq!(envelope_error_kind(resp).as_deref(), Some("remote::engine::invalid_params"), "{resp}");
        assert!(error_message(resp).contains(words), "`{words}`: {resp}");
    };

    refused(&host.interface("interface remote engine set 'lab' --jump 'nowhere'"), "names no engine");
    refused(&host.interface("interface remote engine set 'lab' --jump 'lab'"), "names the entry itself");
    let set = host.interface("interface remote engine set 'lab' --jump 'gate'");
    assert!(!has_error_path(&set), "{set}");
    refused(&host.interface("interface remote engine set 'gate' --jump 'lab'"), "takes a jump of its own");
    refused(&host.interface("interface remote engine unset 'gate'"), "`lab`: jump `gate` names no engine");
    let unset = host.interface("interface remote engine unset 'lab'");
    assert!(!has_error_path(&unset), "{unset}");
    let gone = host.interface("interface remote engine unset 'gate'");
    assert!(!has_error_path(&gone), "the jump goes once nothing jumps through it: {gone}");
}

#[test]
fn a_host_key_refused_names_its_hop() {
    let gate_sshd = Sshd::start_jump();
    let lab_sshd = Sshd::start();
    let account = TestAccount::new();
    let root = tempfile::tempdir().expect("a scratch root");
    let stray = TestKey::new(&root.path().join("stray_ed25519"));
    let gate = gate_sshd.endpoint("gate", &account);
    let mut stray_gate = gate_sshd.endpoint("stray_gate", &account);
    stray_gate.host_keys = vec![stray.public.clone()];
    let mut stray_lab = lab_sshd.endpoint("stray_lab", &account);
    stray_lab.host_keys = vec![stray.public.clone()];
    let engines = [
        through(lab_sshd.endpoint("lab", &account), &stray_gate),
        through(stray_lab, &gate),
        gate,
        stray_gate,
    ];
    let mut host = host_with(root.path(), &engines);

    for (alias, hop) in [("lab", "the jump `stray_gate` at"), ("stray_lab", "through the jump `gate`")] {
        let id = host.request("execute", json!({"host": alias, "definition": PID, "data": "null"}));
        let refused = host.read_id_within(id, FIRST_CALL);
        let kind = envelope_error_kind(&refused);
        assert_eq!(kind.as_deref(), Some("remote::engine::host_key_rejected"), "{alias}: {refused}");
        assert!(error_message(&refused).contains(hop), "{alias} names its hop, `{hop}`: {refused}");
    }
}
