
use sourcetrait_grammar_mcp::guts::{
    TestServer, error_kind, error_text, has_error, nuon, write_tree,
};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary: constructing a TestServer runs
/// the namespace substrate (keypair, equip repo git config), which must not race
/// itself across parallel tests.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

const EXTRA_COLUMNS: &str = "def execute [data: record<extra: record<>>]: nothing -> record<cols: int> { { cols: ($data.extra | columns | length) } }";

#[test]
fn a_nested_open_record_binds_any_record() {
    let s = TESTING.harness();
    let env = s.execute(EXTRA_COLUMNS, "{extra: {a: 1, b: x}}");
    assert_eq!(env["result"], nuon("{cols: 2}"), "an open record field takes any record; got {env}");
}

#[test]
fn a_nested_open_record_binds_an_empty_record() {
    let s = TESTING.harness();
    let env = s.execute(EXTRA_COLUMNS, "{extra: {}}");
    assert_eq!(env["result"], nuon("{cols: 0}"), "got {env}");
}

#[test]
fn a_non_record_in_an_open_record_field_is_refused() {
    let s = TESTING.harness();
    let env = s.execute(EXTRA_COLUMNS, "{extra: 5}");
    assert_eq!(error_kind(&env), Some("data::invalid"), "a non-record `extra` should be refused; got {env}");
}

#[test]
fn an_open_record_at_the_data_top_is_refused() {
    let s = TESTING.harness();
    let env = s.execute("def execute [data: record<>]: nothing -> record<out: int> { { out: 0 } }", "{}");
    assert_eq!(error_kind(&env), Some("definition::invalid"), "got {env}");
}

#[test]
fn an_open_record_in_the_result_is_refused() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<extra: record<>> { { extra: {} } }",
        "null",
    );
    assert_eq!(error_kind(&env), Some("definition::invalid"), "got {env}");
    assert!(
        error_text(&env).contains("refused in a return type"),
        "expected the open-record refusal; got {env}",
    );
}

#[tested]
fn commit_inspect_and_call_open_record_arg_field() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/openlib");
    write_tree(
        &src,
        &[
            ("mod.nu", "export module m\n"),
            ("m/mod.nu", "export use tally\n"),
            (
                "m/tally/mod.nu",
                "export def main [data: record<x: int, extra: record<>>]: nothing -> record<sum: int, extracols: int> {\n    { sum: $data.x, extracols: ($data.extra | columns | length) }\n}\n",
            ),
        ],
    );
    let committed = s.commit("sourcetrait/openlib");
    assert!(!has_error(&committed), "a call-target with a `record<>` arg field should commit; got {committed}");

    let inspected = s.inspect("sourcetrait/openlib:m:tally");
    assert_eq!(
        inspected["result"]["doc"]["signature"].as_str(),
        Some("sourcetrait/openlib:m:tally <x:int,extra:record<>> <sum:int,extracols:int>"),
        "the OPEN record keeps its `record<>` spelling inside the signature - it is \
         the author's literal arg syntax, not a bare `record`; got {inspected}",
    );

    let called = s.call("sourcetrait/openlib:m:tally", "{x: 5, extra: {a: 1, b: 2, c: 3}}");
    assert_eq!(called["result"]["sum"].as_i64(), Some(5), "got {called}");
    assert_eq!(called["result"]["extracols"].as_i64(), Some(3), "got {called}");
}
