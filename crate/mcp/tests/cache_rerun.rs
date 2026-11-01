//! The definition cache behind `rerun`: written before dispatch, so every `execute`
//! call's nonce is a handle, a timed-out one's included.

use sourcetrait_grammar_mcp::guts::{TestServer, error_kind, has_error, nuon};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary: constructing a TestServer runs
/// the namespace substrate (keypair, equip repo git config), which must not race
/// itself across parallel tests.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .setup(|_| TestServer::new())
});

const TRIPLE: &str =
    "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x * 3) } }";

#[test]
fn the_envelope_is_the_nonce_and_the_result_alone() {
    let s = TESTING.harness();
    let env = s.execute(TRIPLE, "{x: 1}");
    assert!(
        env["nonce"].as_str().is_some_and(|n| !n.is_empty()),
        "the envelope carries a non-empty nonce; got {env}",
    );
    let fields: Vec<&String> = env
        .as_record()
        .expect("an envelope record")
        .iter()
        .map(|(k, _)| k)
        .collect();
    assert_eq!(fields, vec!["nonce", "result"], "got {env}");
}

#[test]
fn timeout_then_rerun_recovers() {
    let s = TESTING.harness();
    let timed = s.execute_timeout(
        "def execute [data: record<x: int>]: nothing -> record<out: int> {\n    sleep 400ms\n    { out: ($data.x + 1) }\n}",
        "{x: 5}",
        Some("100ms"),
    );
    assert_eq!(error_kind(&timed), Some("thread::timeout"), "the first call should time out; got {timed}");
    let nonce = timed["nonce"].as_str().expect("a timeout's nonce").to_string();

    // The definition was cached BEFORE dispatch, so the timed-out nonce is a valid
    // rerun handle, and a larger timeout recovers it.
    let recovered = s.rerun_timeout(&nonce, "{x: 5}", Some("5sec"));
    assert_eq!(recovered["result"], nuon("{out: 6}"), "got {recovered}");
}

#[test]
fn rerun_by_nonce_roundtrips_with_new_data() {
    let s = TESTING.harness();
    let first = s.execute(TRIPLE, "{x: 5}");
    assert_eq!(first["result"], nuon("{out: 15}"), "got {first}");
    let nonce = first["nonce"].as_str().expect("nonce").to_string();

    let second = s.rerun(&nonce, "{x: 7}");
    assert_eq!(second["result"], nuon("{out: 21}"), "rerun with new data; got {second}");

    // The rerun's OWN nonce is itself a handle: a rerun caches its definition too.
    let rerun_nonce = second["nonce"].as_str().expect("rerun nonce").to_string();
    assert_ne!(rerun_nonce, nonce, "each eval gets a fresh nonce");
    let third = s.rerun(&rerun_nonce, "{x: 2}");
    assert_eq!(third["result"], nuon("{out: 6}"), "a rerun's nonce must itself be rerunnable; got {third}");
}

#[test]
fn rerun_unknown_nonce_errors() {
    let s = TESTING.harness();
    let env = s.rerun("abcDEF123456", "{x: 0}");
    assert_eq!(error_kind(&env), Some("rerun::body_missing"), "got {env}");
}

#[test]
fn rerun_rejects_non_base62_nonce() {
    let s = TESTING.harness();
    let env = s.rerun("../etc/passwd", "{x: 0}");
    assert!(has_error(&env), "a non-base62 nonce should error; got {env}");
    assert_eq!(error_kind(&env), Some("rerun::invalid_nonce"), "got {env}");
}

#[test]
fn a_stateful_definition_reruns_on_its_lane() {
    let s = TESTING.harness();
    let first = s.execute(
        "def --env execute [data: record<v: string>]: nothing -> record<v: string> {\n    $env.CACHE_RERUN_LANE = $data.v\n    { v: $data.v }\n}",
        "{v: first}",
    );
    assert_eq!(first["result"], nuon("{v: first}"), "got {first}");
    let nonce = first["nonce"].as_str().expect("nonce").to_string();
    let again = s.rerun(&nonce, "{v: again}");
    assert_eq!(again["result"], nuon("{v: again}"), "got {again}");

    let read = s.execute(
        "def --env execute [data: nothing]: nothing -> record<v: string> { { v: $env.CACHE_RERUN_LANE } }",
        "null",
    );
    assert_eq!(
        read["result"],
        nuon("{v: again}"),
        "the rerun ran on the stateful lane, its env write kept; got {read}",
    );
}
