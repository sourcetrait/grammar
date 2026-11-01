//! Domain smoke tests, driven in-process via `guts::TestServer`. The raw-stdio
//! transport (the JSON-RPC handshake + tools/list framing) is a SYSTEM concern
//! covered by `sourcetrait_grammar_tests` (host_run_tool); these exercise what
//! nu / processes / kill actually DO.

use sourcetrait_grammar_mcp::guts::{
    TestServer, error_kind, error_messages, error_text, has_error, nuon,
};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary: constructing a TestServer runs
/// the namespace substrate (keypair, equip repo git config), which must not race
/// itself across parallel tests.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .setup(|_| TestServer::new())
});

const PLUS_ONE: &str =
    "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x + 1) } }";

#[test]
fn a_data_type_mismatch_surfaces() {
    let s = TESTING.harness();
    let env = s.execute(PLUS_ONE, "{x: five}");
    assert_eq!(error_kind(&env), Some("data::invalid"), "got {env}");
}

#[test]
fn a_result_type_mismatch_surfaces() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x | into string) } }",
        "{x: 5}",
    );
    assert!(has_error(&env), "a result of the wrong type should surface as an error; got {env}");
}

#[test]
fn external_command_runs() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<out: string> { { out: (^printf hello | str trim) } }",
        "null",
    );
    assert_eq!(env["result"], nuon("{out: hello}"), "got {env}");
}

#[test]
fn exit_decl_is_unreachable() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<out: int> {\n    exit 1\n    { out: 0 }\n}",
        "null",
    );
    assert!(
        has_error(&env),
        "`exit` should be a disabled decl, not a host-fatal process exit; got {env}"
    );
    assert!(
        error_text(&env).contains("disabled"),
        "`exit` must be shadowed by an erroring decl in-process (no host-fatal exit); got {env}",
    );
}

#[test]
fn timeout_fires_then_recovers() {
    let s = TESTING.harness();
    let timed = s.execute_timeout(
        "def execute [data: nothing]: nothing -> record<out: int> {\n    sleep 5sec\n    { out: 0 }\n}",
        "null",
        Some("200ms"),
    );
    assert_eq!(error_kind(&timed), Some("thread::timeout"), "got {timed}");
    assert!(
        error_messages(&timed).iter().any(|m| m.contains("200")),
        "the timeout message should carry the ms; got {timed}",
    );
    assert!(timed["nonce"].as_str().is_some(), "expected a nonce; got {timed}");

    let env = s.execute(PLUS_ONE, "{x: 7}");
    assert_eq!(env["result"], nuon("{out: 8}"), "a fresh call after a timeout should work; got {env}");
}

#[test]
fn processes_empty_when_idle() {
    // A PRIVATE server: on the shared harness a sibling test's in-flight eval
    // would legitimately show here. Touch the harness first so this private
    // construction never races the module's one-time substrate init.
    let _ = TESTING.harness();
    let s = TestServer::new();
    let env = s.processes();
    assert!(
        env["result"]["processes"].as_list().expect("processes array").is_empty(),
        "expected empty in-flight; got {env}",
    );
}

#[test]
fn kill_unknown_nonce_is_silent_ok() {
    let s = TESTING.harness();
    let env = s.kill("doesnotexist");
    assert!(!has_error(&env), "got {env}");
    assert!(
        env.get("result").is_some_and(|result| result.is_nothing()),
        "a void kill of an unknown nonce answers a null result; got {env}",
    );
}

#[test]
fn plugin_path_resolves() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<path: string> { { path: $nu.plugin-path } }",
        "null",
    );
    let path = env["result"]["path"]
        .as_str()
        .unwrap_or_else(|| panic!("expected string; got {env}"));
    assert!(
        path.ends_with("plugin.msgpackz"),
        "expected $nu.plugin-path to end with plugin.msgpackz; got {path:?}",
    );
}

#[test]
fn tls_crypto_provider_installed() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<out: string> {\n    { out: (try { http get 'https://127.0.0.1:9' | to text } catch {|e| $e.msg }) }\n}",
        "null",
    );
    let out = env["result"]["out"].as_str().unwrap_or_default();
    assert!(!out.is_empty(), "expected a connection error message; got {env}");
    assert!(
        !out.to_lowercase().contains("crypto provider"),
        "provider should be installed; got {out:?}",
    );
}

#[test]
fn multi_call_stability_and_scoping() {
    let s = TESTING.harness();
    for i in 0..10i64 {
        let env = s.execute(PLUS_ONE, &format!("{{x: {i}}}"));
        assert_eq!(env["result"]["out"].as_i64(), Some(i + 1), "call {i}: got {env}");
    }
    // `scope commands` reports LOCAL scope as of nushell 0.115 (PR #18684), so an
    // eval always sees its own `execute` and that name cannot witness leakage.
    // Probe with a name only THIS eval defines instead: visible to its own eval,
    // and absent from a later eval unless the shared base was polluted.
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<defined: int> {\n    def __leak_probe [] { 0 }\n    { defined: (scope commands | where name == '__leak_probe' | length) }\n}",
        "null",
    );
    assert_eq!(
        env["result"]["defined"].as_i64(),
        Some(1),
        "a block-local def is visible to its own eval under 0.115 local scope; got {env}",
    );
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<leaked: int> { { leaked: (scope commands | where name == '__leak_probe' | length) } }",
        "null",
    );
    assert_eq!(
        env["result"]["leaked"].as_i64(),
        Some(0),
        "an earlier eval's def must not reach a later eval's engine; got {env}",
    );
}
