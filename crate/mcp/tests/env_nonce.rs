
use sourcetrait_grammar_mcp::guts::{TestServer, has_error, write_tree};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary: constructing a TestServer runs
/// the namespace substrate (keypair, equip repo git config), which must not race
/// itself across parallel tests.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

#[test]
fn a_stateless_body_sees_the_envelope_nonce() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<seen: string> { {seen: $env.NONCE} }",
        "null",
    );
    let nonce = env["nonce"].as_str().expect("envelope nonce");
    assert!(!nonce.is_empty(), "envelope nonce should be non-empty");
    assert_eq!(env["result"]["seen"].as_str(), Some(nonce), "$env.NONCE should equal the envelope nonce; got {env}");
}

#[test]
fn a_nested_helper_inherits_the_nonce() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<seen: string> {\n    def grab [] { $env.NONCE }\n    {seen: (grab)}\n}",
        "null",
    );
    let nonce = env["nonce"].as_str().expect("envelope nonce");
    assert_eq!(env["result"]["seen"].as_str(), Some(nonce), "a nested helper should inherit $env.NONCE; got {env}");
}

#[tested]
fn call_target_sees_nonce() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/noncelib");
    write_tree(
        &src,
        &[
            ("mod.nu", "export module m\n"),
            ("m/mod.nu", "export use whoami\n"),
            (
                "m/whoami/mod.nu",
                "export def main [data: record<noop: int>]: nothing -> record<seen: string> { { seen: $env.NONCE } }\n",
            ),
        ],
    );
    let committed = s.commit("sourcetrait/noncelib");
    assert!(!has_error(&committed), "commit should succeed; got {committed}");
    let env = s.call("sourcetrait/noncelib:m:whoami", "{noop: 0}");
    let nonce = env["nonce"].as_str().expect("envelope nonce");
    assert_eq!(env["result"]["seen"].as_str(), Some(nonce), "a committed call-target should read $env.NONCE; got {env}");
}

#[test]
fn a_stateful_body_sees_the_envelope_nonce() {
    let s = TESTING.harness();
    let env = s.execute(
        "def --env execute [data: nothing]: nothing -> record<seen: string> { {seen: $env.NONCE} }",
        "null",
    );
    let nonce = env["nonce"].as_str().expect("envelope nonce");
    assert_eq!(env["result"]["seen"].as_str(), Some(nonce), "a stateful body's $env.NONCE should equal the envelope nonce; got {env}");
}

#[test]
fn the_stateful_nonce_is_fresh_per_call_and_env_persists() {
    let s = TESTING.harness();
    let a = s.execute(
        "def --env execute [data: nothing]: nothing -> record<seen: string> {\n    $env.KEEP = $env.NONCE\n    {seen: $env.NONCE}\n}",
        "null",
    );
    let nonce_a = a["nonce"].as_str().expect("A nonce").to_string();
    assert_eq!(a["result"]["seen"].as_str(), Some(nonce_a.as_str()), "{a}");

    let b = s.execute(
        "def --env execute [data: nothing]: nothing -> record<seen: string, keep: string> {\n    {seen: $env.NONCE, keep: $env.KEEP}\n}",
        "null",
    );
    let nonce_b = b["nonce"].as_str().expect("B nonce").to_string();
    assert_ne!(nonce_a, nonce_b, "each call gets a distinct nonce");
    assert_eq!(b["result"]["seen"].as_str(), Some(nonce_b.as_str()), "B sees B's nonce; got {b}");
    assert_eq!(b["result"]["keep"].as_str(), Some(nonce_a.as_str()), "A's $env.KEEP write should persist into B; got {b}");
}
