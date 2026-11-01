//! The `execute` tool end to end through the in-process harness: its lanes, its
//! checks, its timeout, and `rerun` over its definitions.

use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_mcp::guts::{NuVal, TestServer, error_kind, error_messages, has_error, nuon};

static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

/// A record's field names, sorted.
fn keys(record: &NuVal) -> Vec<&str> {
    let mut keys: Vec<&str> = record
        .as_record()
        .unwrap_or_else(|| panic!("not a record: {record}"))
        .iter()
        .map(|(k, _)| k.as_str())
        .collect();
    keys.sort();
    keys
}

fn nonce_of(env: &NuVal) -> String {
    env["nonce"]
        .as_str()
        .unwrap_or_else(|| panic!("no nonce in {env}"))
        .to_string()
}

#[test]
fn a_record_round_trips_on_the_stateless_pool() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: record<a: int>]: nothing -> record<b: int> { {b: ($data.a + 1)} }",
        "{a: 41}",
    );
    assert!(!has_error(&env), "{env}");
    assert_eq!(env["result"], nuon("{b: 42}"));
    assert!(!nonce_of(&env).is_empty());
}

#[test]
fn a_table_round_trips() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: table<n: int>]: nothing -> table<n: int, double: int> {\n    $data | each {|row| {n: $row.n, double: ($row.n * 2)} }\n}",
        "[[n]; [1] [2]]",
    );
    assert!(!has_error(&env), "{env}");
    assert_eq!(env["result"], nuon("[{n: 1, double: 2}, {n: 2, double: 4}]"));
}

#[test]
fn a_nothing_definition_takes_null_and_answers_null() {
    let s = TESTING.harness();
    let env = s.execute("def execute [data: nothing]: nothing -> nothing { }", "null");
    assert!(!has_error(&env), "{env}");
    assert!(env["result"].is_nothing(), "{env}");
}

#[test]
fn the_stateful_lane_keeps_env_and_cwd_between_calls() {
    let s = TESTING.harness();
    let set = s.execute(
        "def --env execute [data: record<v: string>]: nothing -> nothing { $env.NUNU_PROBE = $data.v }",
        "{v: kept}",
    );
    assert!(!has_error(&set), "{set}");
    let get = s.execute(
        "def --env execute [data: nothing]: nothing -> record<v: string> { {v: $env.NUNU_PROBE} }",
        "null",
    );
    assert_eq!(get["result"], nuon("{v: kept}"), "{get}");
}

#[test]
fn the_stateless_pool_keeps_nothing_between_calls() {
    let s = TESTING.harness();
    let set = s.execute(
        "def execute [data: nothing]: nothing -> nothing { $env.NUNU_LOST = 'yes' }",
        "null",
    );
    assert!(!has_error(&set), "{set}");
    let get = s.execute(
        "def execute [data: nothing]: nothing -> record<seen: bool> { {seen: ('NUNU_LOST' in $env)} }",
        "null",
    );
    assert_eq!(get["result"], nuon("{seen: false}"), "{get}");
}

#[test]
fn undeclared_data_is_refused_before_dispatch_with_a_nonce() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: record<a: int>]: nothing -> record<a: int> { $data }",
        "{a: 1, b: 2}",
    );
    assert_eq!(error_kind(&env), Some("data::invalid"), "{env}");
    assert_eq!(error_messages(&env), vec!["data.b: an undeclared field".to_string()]);
    assert!(!nonce_of(&env).is_empty());
}

#[test]
fn missing_data_is_refused() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: record<a: int, b: int>]: nothing -> record<a: int> { {a: $data.a} }",
        "{b: 2}",
    );
    assert_eq!(error_kind(&env), Some("data::invalid"), "{env}");
    assert_eq!(error_messages(&env), vec!["data.a: a declared field is missing".to_string()]);
}

#[test]
fn data_of_the_wrong_type_is_refused() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: record<a: int>]: nothing -> record<a: int> { $data }",
        "{a: one}",
    );
    assert_eq!(error_kind(&env), Some("data::invalid"), "{env}");
    assert_eq!(error_messages(&env), vec!["data.a: expected int, got string".to_string()]);
}

#[test]
fn data_that_is_not_nuon_is_refused() {
    let s = TESTING.harness();
    let env = s.execute("def execute [data: record<a: int>]: nothing -> record<a: int> { $data }", "{a: ");
    assert_eq!(error_kind(&env), Some("data::invalid"), "{env}");
    let message = &error_messages(&env)[0];
    assert!(
        message.starts_with("data is not NUON: ") && message.contains("; error when parsing: "),
        "the refusal carries the parse error's own words, not the nuon crate's generic line; {env}",
    );
    let env = s.execute("def execute [data: nothing]: nothing -> nothing { }", "");
    assert_eq!(error_kind(&env), Some("data::invalid"), "{env}");
}

#[test]
fn an_undeclared_result_field_is_refused_with_the_nonce() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<a: int> { {a: 1, b: 2} }",
        "null",
    );
    assert_eq!(error_kind(&env), Some("result::invalid"), "{env}");
    assert_eq!(error_messages(&env), vec!["result.b: an undeclared field".to_string()]);
}

#[test]
fn a_result_of_the_wrong_shape_is_refused_when_it_runs() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<a: int> { '[1]' | from nuon }",
        "null",
    );
    assert_eq!(error_kind(&env), Some("result::invalid"), "{env}");
    assert_eq!(error_messages(&env), vec!["result: expected record, got list<int>".to_string()]);
}

#[test]
fn a_statically_wrong_result_is_refused_at_parse() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: record<n: int>]: nothing -> record<a: int> {\n    let rows = [$data.n]\n    $rows\n}",
        "{n: 1}",
    );
    assert_eq!(error_kind(&env), Some("definition::invalid"), "{env}");
}

#[test]
fn a_contract_violation_is_refused_as_a_definition_row() {
    let s = TESTING.harness();
    let env = s.execute("def execute [data: list<int>]: nothing -> nothing { }", "[1]");
    assert_eq!(error_kind(&env), Some("definition::invalid"), "{env}");
}

#[test]
fn the_timeout_reaches_a_body_that_declares_it() {
    let s = TESTING.harness();
    let env = s.execute_timeout(
        "def execute [data: nothing, timeout?: duration]: nothing -> record<within: bool> {\n    {within: ($timeout == 30sec)}\n}",
        "null",
        Some("30sec"),
    );
    assert_eq!(env["result"], nuon("{within: true}"), "{env}");
    let env = s.execute(
        "def execute [data: nothing, timeout?: duration]: nothing -> record<within: bool> {\n    {within: ($timeout == 120sec)}\n}",
        "null",
    );
    assert_eq!(env["result"], nuon("{within: true}"), "the default reaches it; {env}");
}

#[test]
fn a_timeout_fires() {
    let s = TESTING.harness();
    let env = s.execute_timeout(
        "def execute [data: nothing]: nothing -> nothing { sleep 10sec }",
        "null",
        Some("300ms"),
    );
    assert_eq!(error_kind(&env), Some("thread::timeout"), "{env}");
}

#[test]
fn a_bad_timeout_is_refused() {
    let s = TESTING.harness();
    for timeout in ["5", "soon", "-1sec", "0sec"] {
        let env = s.execute_timeout("def execute [data: nothing]: nothing -> nothing { }", "null", Some(timeout));
        assert_eq!(error_kind(&env), Some("timeout::invalid"), "{timeout}: {env}");
    }
    let five = s.execute_timeout("def execute [data: nothing]: nothing -> nothing { }", "null", Some("5"));
    assert_eq!(
        error_messages(&five),
        vec!["`5` is typed int, not a positive duration such as 30sec".to_string()],
        "the refusal names the value's type; got {five}",
    );
}

#[test]
fn rerun_evaluates_a_cached_definition_with_fresh_data() {
    let s = TESTING.harness();
    let first = s.execute(
        "def execute [data: record<a: int>]: nothing -> record<b: int> { {b: ($data.a * 10)} }",
        "{a: 1}",
    );
    assert_eq!(first["result"], nuon("{b: 10}"), "{first}");
    let source = nonce_of(&first);
    let again = s.rerun(&source, "{a: 7}");
    assert_eq!(again["result"], nuon("{b: 70}"), "{again}");
    let fresh = nonce_of(&again);
    assert_ne!(fresh, source, "a rerun mints its own nonce");
    let chained = s.rerun(&fresh, "{a: 2}");
    assert_eq!(chained["result"], nuon("{b: 20}"), "a rerun's nonce is a handle too; {chained}");
    let refused = s.rerun(&source, "{a: x}");
    assert_eq!(error_kind(&refused), Some("data::invalid"), "{refused}");
}

#[test]
fn a_non_eval_tool_answers_the_same_envelope() {
    let s = TESTING.harness();
    let ok = s.purviews();
    assert_eq!(keys(&ok), vec!["nonce", "result"], "{ok}");
    assert!(ok["result"]["current"].as_list().is_some(), "{ok}");

    let refused = s.inspect("sourcetrait/nosuchrig");
    assert_eq!(keys(&refused), vec!["error", "nonce"], "{refused}");
    let rows = refused["error"].as_list().expect("an error table");
    assert_eq!(rows.len(), 1, "{refused}");
    assert_eq!(keys(&rows[0]), vec!["kind", "message", "source"], "{refused}");
    assert_eq!(error_kind(&refused), Some("equip::not_registered"), "{refused}");
}

#[test]
fn a_parse_error_reads_in_nushells_words() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<ok: bool> { error make {msg: 'boom'} }",
        "null",
    );
    assert_eq!(error_kind(&env), Some("definition::invalid"), "{env}");
    assert_eq!(
        error_messages(&env),
        vec![
            "parse error: Command output doesn't match record<ok: bool>: \
             expected record<ok: bool>, but command outputs error"
                .to_string()
        ]
    );
}

#[test]
fn a_shell_error_keeps_its_label_and_help() {
    let s = TESTING.harness();
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<n: int> { {n: (plugin list | length)} }",
        "null",
    );
    assert_eq!(error_kind(&env), Some("thread::returned_error"), "{env}");
    let messages = error_messages(&env);
    assert!(
        messages[0].starts_with("eval: External command failed: ") && messages[0].contains("plugin"),
        "{env}"
    );
}

#[test]
fn rerun_refuses_a_missing_or_malformed_nonce() {
    let s = TESTING.harness();
    let env = s.rerun("NoSuchNonce1", "null");
    assert_eq!(error_kind(&env), Some("rerun::body_missing"), "{env}");
    let env = s.rerun("../escape", "null");
    assert_eq!(error_kind(&env), Some("rerun::invalid_nonce"), "{env}");
}
