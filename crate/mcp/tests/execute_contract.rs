//! The execute contract over `execute` definitions, through `guts::analyze_definition`.

use sourcetrait_grammar_mcp::guts::{NuVal, analyze_definition};

fn accepted(definition: &str) -> NuVal {
    analyze_definition(definition)
        .unwrap_or_else(|rows| panic!("refused `{definition}`: {rows:?}"))
}

fn refused(definition: &str) -> Vec<NuVal> {
    match analyze_definition(definition) {
        Ok(summary) => panic!("accepted `{definition}` as {summary}"),
        Err(rows) => rows,
    }
}

fn kinds(rows: &[NuVal]) -> Vec<&str> {
    rows.iter().map(|row| row["kind"].as_str().expect("kind")).collect()
}

fn refused_as_definition(definition: &str, needle: &str) {
    let rows = refused(definition);
    assert!(
        rows.iter().all(|row| row["kind"] == "definition::invalid"),
        "`{definition}` refused under other kinds: {rows:?}",
    );
    assert!(
        rows.iter()
            .any(|row| row["message"].as_str().is_some_and(|m| m.contains(needle))),
        "no row of `{definition}` says `{needle}`: {rows:?}",
    );
}

#[test]
fn a_record_definition_runs_stateless() {
    let summary =
        accepted("def execute [data: record<a: int>]: nothing -> record<b: int> { {b: $data.a} }");
    assert_eq!(summary["lane"], "stateless");
    assert_eq!(summary["data_type"], "record<a: int>");
    assert_eq!(summary["result_type"], "record<b: int>");
    assert_eq!(summary["takes_timeout"], false);
}

#[test]
fn an_env_definition_runs_stateful() {
    let summary = accepted(
        "def --env execute [data: record<dir: directory>]: nothing -> nothing { cd $data.dir }",
    );
    assert_eq!(summary["lane"], "stateful");
    assert_eq!(summary["data_type"], "record<dir: directory>");
    assert_eq!(summary["result_type"], "nothing");
}

#[test]
fn tables_and_the_timeout_parameter_stand() {
    let summary = accepted(
        "def execute [data: table<n: int>, timeout?: duration]: nothing -> table<m: int> {\n\
         \x20   $data | each {|row| {m: $row.n} }\n}",
    );
    assert_eq!(summary["data_type"], "table<n: int>");
    assert_eq!(summary["result_type"], "table<m: int>");
    assert_eq!(summary["takes_timeout"], true);
}

#[test]
fn an_open_record_stands_nested_in_data() {
    let summary = accepted(
        "def execute [data: record<fill: record<>, rows: table<>>]: nothing -> record<ok: bool> {\n\
         \x20   {ok: true}\n}",
    );
    assert_eq!(summary["data_type"], "record<fill: record<>, rows: table<>>");
}

#[test]
fn comments_stand_beside_the_definition() {
    accepted("# One line of documentation.\ndef execute [data: nothing]: nothing -> nothing { }\n");
}

#[test]
fn a_second_top_level_element_is_refused() {
    refused_as_definition(
        "def helper [] { 1 }\ndef execute [data: nothing]: nothing -> nothing { }",
        "exactly one top-level",
    );
}

#[test]
fn another_command_name_is_refused() {
    refused_as_definition("def go [data: nothing]: nothing -> nothing { }", "named `execute`");
}

#[test]
fn export_def_is_refused() {
    refused_as_definition(
        "export def execute [data: nothing]: nothing -> nothing { }",
        "not `export def`",
    );
}

#[test]
fn def_wrapped_is_refused() {
    refused_as_definition(
        "def --wrapped execute [data: nothing]: nothing -> nothing { }",
        "--wrapped",
    );
}

#[test]
fn a_missing_infix_signature_is_refused() {
    refused_as_definition("def execute [data: record<a: int>] { $data }", "infix signature");
}

#[test]
fn an_input_other_than_nothing_is_refused() {
    refused_as_definition(
        "def execute [data: record<a: int>]: any -> record<a: int> { $data }",
        "infix signature",
    );
}

#[test]
fn a_second_required_parameter_is_refused() {
    refused_as_definition(
        "def execute [data: record<a: int>, other: int]: nothing -> record<a: int> { $data }",
        "one required parameter",
    );
}

#[test]
fn a_timeout_of_another_type_is_refused() {
    refused_as_definition(
        "def execute [data: nothing, timeout?: int]: nothing -> nothing { }",
        "typed `duration`",
    );
}

#[test]
fn a_timeout_default_is_refused() {
    refused_as_definition(
        "def execute [data: nothing, timeout: duration = 5sec]: nothing -> nothing { }",
        "takes no default",
    );
}

#[test]
fn a_flag_is_refused() {
    refused_as_definition(
        "def execute [data: nothing, --verbose]: nothing -> nothing { }",
        "takes no flags",
    );
}

#[test]
fn a_rest_parameter_is_refused() {
    refused_as_definition(
        "def execute [data: nothing, ...more: int]: nothing -> nothing { }",
        "no rest parameter",
    );
}

#[test]
fn an_open_record_at_the_top_of_data_is_refused() {
    refused_as_definition(
        "def execute [data: record<>]: nothing -> nothing { }",
        "stands only nested",
    );
}

#[test]
fn untyped_data_is_refused() {
    refused_as_definition("def execute [data]: nothing -> nothing { }", "`any`");
}

#[test]
fn a_list_at_the_top_of_data_is_refused() {
    refused_as_definition(
        "def execute [data: list<int>]: nothing -> nothing { }",
        "typed `nothing`, `record<...>`, or `table<...>`",
    );
}

#[test]
fn a_nested_list_of_tables_is_refused() {
    refused_as_definition(
        "def execute [data: record<t: list<table<a: int>>>]: nothing -> nothing { }",
        "list of tables",
    );
}

#[test]
fn an_open_record_in_the_return_is_refused() {
    refused_as_definition(
        "def execute [data: nothing]: nothing -> record<x: record<>> { {x: {}} }",
        "open `record<>`",
    );
}

#[test]
fn a_list_return_is_refused() {
    refused_as_definition(
        "def execute [data: nothing]: nothing -> list<int> { [1] }",
        "typed `nothing`, `record<...>`, or `table<...>`",
    );
}

#[test]
fn an_any_return_is_refused() {
    refused_as_definition(
        "def execute [data: nothing]: nothing -> any { 1 }",
        "typed `nothing`, `record<...>`, or `table<...>`",
    );
}

#[test]
fn a_parse_error_is_refused_with_its_position() {
    let rows = refused("def execute [data: nothing]: nothing -> nothing {\n    ( \n}");
    assert_eq!(kinds(&rows)[0], "definition::invalid");
    assert!(rows[0]["source"]["position"][0].as_u64().unwrap() >= 1, "{rows:?}");
}

#[test]
fn the_body_is_linted() {
    let rows = refused(
        "def execute [data: nothing]: nothing -> nothing {\n    open \"/srv/x.txt\" | ignore\n}",
    );
    assert_eq!(kinds(&rows), vec!["lint::hardcoded_variable"]);
    assert_eq!(rows[0]["source"]["position"][0], 2);
    let rows = refused(
        "def execute [data: nothing]: nothing -> nothing {\n    ^grep foo | ignore\n}",
    );
    assert_eq!(kinds(&rows), vec!["lint::denied_command"]);
}
