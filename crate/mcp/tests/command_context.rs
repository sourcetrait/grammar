
use sourcetrait_grammar_mcp::guts::{TestServer, lint_engine_reuses_until_the_registry_moves, nuon};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary: constructing a TestServer runs
/// the namespace substrate (keypair, equip repo git config), which must not race
/// itself across parallel tests.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .setup(|_| TestServer::new())
});

/// The validator engine is rebuilt on a plugin-registry change, not on every
/// call. Integration rather than unit: constructing one builds a whole shell
/// command context and reads the registry off disk.
#[test]
fn lint_engine_does_not_rebuild_when_the_registry_is_unchanged() {
    assert!(
        lint_engine_reuses_until_the_registry_moves(),
        "current() must hand back the SAME engine while the plugin registry is \
         unchanged; rebuilding per call would pay a full command-context build \
         on every lint and every commit",
    );
}

/// The stateless pool carries nu-cmd-extra (bits / str-case) but NOT the
/// `plugin *` admin family; the stateful lane carries both.
#[test]
fn the_stateless_pool_has_extra_and_lacks_plugin() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<bits: bool, snake: bool, plugin: bool, bits_and: int> {\n    let names = (scope commands | get name)\n    { bits: ('bits and' in $names), snake: ('str snake-case' in $names), plugin: ('plugin list' in $names), bits_and: (5 | bits and 3) }\n}",
        "null",
    );
    assert_eq!(
        env["result"],
        nuon("{bits: true, snake: true, plugin: false, bits_and: 1}"),
        "the pool has `bits and` and `str snake-case`, never `plugin *`; got {env}",
    );
}

#[test]
fn the_stateful_lane_has_extra_and_plugin() {
    let s = TESTING.harness();
    let env = s.execute(
        "def --env execute [data: nothing]: nothing -> record<bits: bool, plugin: bool> {\n    let names = (scope commands | get name)\n    { bits: ('bits and' in $names), plugin: ('plugin list' in $names) }\n}",
        "null",
    );
    assert_eq!(
        env["result"],
        nuon("{bits: true, plugin: true}"),
        "the stateful lane has `bits and` and `plugin *`; got {env}",
    );
}
