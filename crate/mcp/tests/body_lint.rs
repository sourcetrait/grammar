//! The lint at the `nu` tool's boundary: a definition breaking a rule never runs,
//! on either lane. `tests/lint_rules.rs` covers the rules themselves.

use sourcetrait_grammar_mcp::guts::{TestServer, has_kind, nuon};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary: constructing a TestServer runs
/// the namespace substrate (keypair, equip repo git config), which must not race
/// itself across parallel tests.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .setup(|_| TestServer::new())
});

#[test]
fn a_hardcoded_path_is_refused() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<out: int> {\n    let p = \"/srv/proj/x\"\n    { out: 0 }\n}",
        "null",
    );
    assert!(has_kind(&env, "lint::hardcoded_variable"), "got {env}");
}

#[test]
fn a_denied_external_is_refused() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<out: int> {\n    let x = (^awk '{print $1}' | str trim)\n    { out: 0 }\n}",
        "null",
    );
    assert!(has_kind(&env, "lint::denied_command"), "got {env}");
}

#[test]
fn a_clean_definition_runs() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x + 1) } }",
        "{x: 5}",
    );
    assert_eq!(env["result"], nuon("{out: 6}"), "got {env}");
}

#[test]
fn violations_aggregate() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<out: int> {\n    ^awk 'x'\n    cd \"/a/b\"\n    { out: 0 }\n}",
        "null",
    );
    assert!(has_kind(&env, "lint::denied_command"), "got {env}");
    assert!(has_kind(&env, "lint::hardcoded_variable"), "got {env}");
}

#[test]
fn the_stateful_lane_is_linted_too() {
    let s = TESTING.harness();
    let env = s.execute(
        "def --env execute [data: nothing]: nothing -> record<out: int> {\n    let p = \"/srv/x\"\n    { out: 0 }\n}",
        "null",
    );
    assert!(has_kind(&env, "lint::hardcoded_variable"), "got {env}");
    let env = s.execute(
        "def --env execute [data: nothing]: nothing -> record<out: int> {\n    let x = (^awk 'x' | str trim)\n    { out: 0 }\n}",
        "null",
    );
    assert!(has_kind(&env, "lint::denied_command"), "got {env}");
}

#[test]
fn a_well_known_system_path_passes() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<seen: bool> { { seen: ('/dev/null' | path exists) } }",
        "null",
    );
    assert_eq!(env["result"], nuon("{seen: true}"), "got {env}");
}
