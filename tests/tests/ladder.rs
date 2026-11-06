//! A real host's grammar-ssh held to the ladder's checks: a connection held
//! across idle minutes, calls, a rerun, and bed on it, and its close leaving
//! no cache. Ignored by default, its engine named by the environment.
#![cfg(unix)]

use std::time::Duration;

use serde_json::{Value, json};
use sourcetrait_grammar_tests::*;

/// The `[[engine]]` tables a remotes file holds, the engine under test aliased `ladder`.
const ENGINE_VAR: &str = "GRAMMAR_LADDER_ENGINES";

/// How long the session idles between its calls, in seconds; 180 when unset.
const IDLE_VAR: &str = "GRAMMAR_LADDER_IDLE_SECS";

/// How long a call may take, a session's open and grammar-ssh's start among it.
const CALL_WAIT: Duration = Duration::from_secs(180);

/// A definition answering the process it ran in.
const PID: &str = "def execute [data: nothing]: nothing -> record<pid: int> { {pid: $nu.pid} }";

/// A tool's call on the engine under test, its wait a session's open.
fn on_ladder(host: &mut Host, tool: &str, mut args: Value) -> Value {
    args["host"] = json!("ladder");
    let id = host.request(tool, args);
    host.read_id_within(id, CALL_WAIT)
}

/// A response's result, its error envelope a panic.
fn answered(resp: &Value) -> NuVal {
    let envelope = extract_envelope(resp).unwrap_or_else(|| panic!("a result: {}", error_message(resp)));
    envelope["result"].clone()
}

/// The pid a `PID` call answered, and its nonce.
fn pid_of(resp: &Value) -> (i64, String) {
    let envelope = extract_envelope(resp).unwrap_or_else(|| panic!("a result: {}", error_message(resp)));
    let pid = match envelope["result"].get("pid") {
        Some(NuVal::Int(pid)) => *pid,
        other => panic!("a pid, not {other:?}"),
    };
    (pid, envelope["nonce"].as_str().expect("its nonce").to_string())
}

#[test]
#[ignore = "a real host's engine, named by GRAMMAR_LADDER_ENGINES"]
fn a_real_engine_holds_across_idle_and_its_close_leaves_no_cache() {
    let engines = std::env::var(ENGINE_VAR).unwrap_or_else(|_| panic!("{ENGINE_VAR} names the engines"));
    let idle = std::env::var(IDLE_VAR).ok().and_then(|s| s.parse().ok()).unwrap_or(180);
    let root = tempfile::tempdir().expect("a scratch root");
    let agent = root.path().join("agent");
    write_source(&agent, ".grammar/remotes.toml", &engines);
    let mut host = Host::spawn_in(&root.path().join("scratch"), &[], &agent);

    let opened = answered(&on_ladder(&mut host, "interface", json!({"subcommand": "remote engine connect 'ladder'"})));
    let columns: Vec<&str> = opened.as_record().unwrap_or_default().iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(columns, ["nom"], "a connect answers its engine's nom alone: {opened}");
    let nom = opened["nom"].as_str().expect("its engine's nom").to_string();
    let (first, nonce) = pid_of(&on_ladder(&mut host, "execute", json!({"definition": PID, "data": "null"})));

    std::thread::sleep(Duration::from_secs(idle));
    let (held, _) = pid_of(&on_ladder(&mut host, "execute", json!({"definition": PID, "data": "null"})));
    assert_eq!(held, first, "the connection holds across {idle} s idle, one grammar-ssh serving");
    let (rerun, _) = pid_of(&on_ladder(&mut host, "rerun", json!({"nonce": nonce, "data": "null"})));
    assert_eq!(rerun, first, "a rerun on the same grammar-ssh");
    let config = answered(&on_ladder(&mut host, "bed", json!({"command": "grmr", "subcommand": "config list"})));
    assert!(config.as_list().is_some(), "bed reads the config through the session: {config}");
    let traced = answered(&on_ladder(&mut host, "bed", json!({"command": "grmr", "subcommand": "dbg {ladder: true}"})));
    assert!(traced.is_nothing(), "{traced}");
    let row = answered(&host.interface("remote engine get 'ladder'"));
    assert_eq!((row["state"].as_str(), row["nom"].as_str()), (Some("open"), Some(nom.as_str())), "{row}");

    let closed = answered(&on_ladder(&mut host, "interface", json!({"subcommand": "remote engine close 'ladder'"})));
    assert!(closed.is_nothing(), "{closed}");
    let (again, _) = pid_of(&on_ladder(&mut host, "execute", json!({"definition": PID, "data": "null"})));
    assert_ne!(again, first, "a call after the close opens a new grammar-ssh, nothing told");
    let gone = on_ladder(&mut host, "rerun", json!({"nonce": nonce, "data": "null"}));
    assert_eq!(
        envelope_error_kind(&gone).as_deref(),
        Some("rerun::body_missing"),
        "the first session's cache went with it: {gone}",
    );
    let _ = on_ladder(&mut host, "interface", json!({"subcommand": "remote engine close 'ladder'"}));
}
