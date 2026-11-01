//! The embedded API (`grimm *`) - decls registered per-eval into the working set
//! `eval_in_process` parses with, so they exist ONLY inside an `execute` definition or a
//! call's body and carry that call's own log dir.

use sourcetrait_grammar_mcp::guts::{NuVal, TestServer, error_text, has_error, nuon, write_tree};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary: constructing a TestServer runs
/// the namespace substrate (keypair, equip repo git config), which must not race
/// itself across parallel tests.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

/// A definition taking nothing and answering `{ok: true}` after `body`.
fn ok_after(body: &str) -> String {
    format!("def execute [data: nothing]: nothing -> record<ok: bool> {{\n{body}\n{{ ok: true }}\n}}")
}

/// Read `debug.nuonl` back THROUGH nushell, so the assertion covers the round-trip
/// (the file parses as NUON) and not merely the bytes we wrote.
fn read_back(
    s: &TestServer,
    nonce: &str,
    result_type: &str,
    body: &str,
) -> NuVal {
    let path = s.execute_log_dir(nonce).join("debug.nuonl");
    s.execute(
        &format!("def execute [data: record<path: string>]: nothing -> {result_type} {{\n{body}\n}}"),
        &format!("{{path: '{}'}}", path.to_str().expect("utf-8 path")),
    )
}

#[test]
fn dbg_appends_one_line_per_call() {
    let s = TESTING.harness();
    let env = s.execute(&ok_after("grimm dbg {a: 1, b: 'two'}\ngrimm dbg [[x, y]; [1, 2]]"), "null");
    assert!(!has_error(&env), "the definition should run; got {env}");
    let nonce = env["nonce"].as_str().expect("nonce");

    let back = read_back(
        s,
        nonce,
        "record<count: int, first_a: int, second_x: int>",
        "let recs = (open --raw $data.path | decode | lines | each {|l| $l | from nuon })\n\
         { count: ($recs | length), first_a: ($recs | get 0 | get a), second_x: ($recs | get 1 | get 0.x) }",
    );
    assert_eq!(
        back["result"],
        nuon("{count: 2, first_a: 1, second_x: 1}"),
        "one line per call; got {back}",
    );
}

/// `channel send` is the real notification lane, so with no channel open it REFUSES
/// rather than quietly writing a file. A written record nobody was told about is worse
/// than no record.
#[test]
fn channel_send_without_a_channel_refuses() {
    let s = TESTING.harness();
    let env = s.execute(&ok_after("grimm channel send 'foo/bar/Car' {state: 'done'}"), "null");
    assert!(has_error(&env), "an unopened channel must refuse; got {env}");
    let text = error_text(&env).to_lowercase();
    assert_eq!(
        text.matches("channel is not open").count(),
        1,
        "the refusal should name the cause, once; got {}",
        error_text(&env),
    );
}

/// `to nuon` emits a newline INSIDE a string raw, which would split one record
/// across lines and break the nuonl invariant. The escape must survive a
/// round-trip, and must not disturb the fields after it.
#[test]
fn a_record_with_an_embedded_newline_stays_one_line() {
    let s = TESTING.harness();
    let env = s.execute(
        &ok_after("grimm dbg {msg: ('line one' + (char nl) + 'line two'), tail: 9}"),
        "null",
    );
    assert!(!has_error(&env), "got {env}");
    let nonce = env["nonce"].as_str().expect("nonce");

    let back = read_back(
        s,
        nonce,
        "record<lines: int, msg: string, tail: int>",
        "let ls = (open --raw $data.path | decode | lines)\n\
         let rec = ($ls | first | from nuon)\n\
         { lines: ($ls | length), msg: $rec.msg, tail: $rec.tail }",
    );
    assert_eq!(back["result"]["lines"].as_i64(), Some(1), "must not span lines; got {back}");
    assert_eq!(
        back["result"]["msg"].as_str(),
        Some("line one\nline two"),
        "the escaped newline must round-trip to the original string; got {back}",
    );
    assert_eq!(
        back["result"]["tail"].as_i64(),
        Some(9),
        "the escape must not disturb following fields; got {back}",
    );
}

/// The `mcp/` and `remote/` RESERVATIONS. A definition is not the host, so it must
/// not be able to stamp a model the host reserves - otherwise it could forge a control
/// packet, and provenance would stop being checkable from the model path alone.
///
/// Refused BEFORE the channel state is consulted, so the answer does not depend on
/// whether a channel happens to be open.
#[test]
fn a_definition_cannot_claim_a_host_reservation() {
    let s = TESTING.harness();
    for model in [
        "mcp/channel/Open",
        "mcp/supervisor/CpuWarning",
        "mcp/anything",
        "remote/channel/Sent",
        "remote/anything",
    ] {
        let env = s.execute(&ok_after(&format!("grimm channel send '{model}' {{x: 1}}")), "null");
        assert!(has_error(&env), "`{model}` should be refused; got {env}");
        assert!(
            error_text(&env).contains("reserved"),
            "the refusal should name the reservation; got {}",
            error_text(&env),
        );
    }
}

/// A literal is rejected by the signature's `oneof<record, table>` shape at parse
/// time; a DYNAMIC value reaches `run` unchecked, which is what the runtime guard
/// is for. Both must fail.
#[test]
fn a_non_record_is_rejected() {
    let s = TESTING.harness();
    for body in ["grimm dbg 5", "let x = 5\ngrimm dbg $x"] {
        let env = s.execute(&ok_after(body), "null");
        assert!(has_error(&env), "`{body}` should be rejected; got {env}");
        let text = error_text(&env).to_lowercase();
        assert!(
            text.contains("record") || text.contains("table"),
            "the rejection should name the accepted shapes; got {}",
            error_text(&env),
        );
    }
}

/// The contract's parse and the rig validator's know the `grimm` decls as an eval
/// does, so a table passed to one is a table there too, never an external's
/// argument that fails to parse.
#[tested]
fn a_rig_passing_a_table_to_grimm_commits_and_runs() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/grimmrig");
    write_tree(
        &src,
        &[
            ("mod.nu", "export module m\n"),
            ("m/mod.nu", "export use trace\n"),
            (
                "m/trace/mod.nu",
                "export def main [data: nothing]: nothing -> record<ok: bool> {\n    grimm dbg [[x, y]; [1, 2]]\n    { ok: true }\n}\n",
            ),
        ],
    );
    let committed = s.commit("sourcetrait/grimmrig");
    assert!(!has_error(&committed), "got {committed}");
    let called = s.call("sourcetrait/grimmrig:m:trace", "null");
    assert_eq!(called["result"], nuon("{ok: true}"), "got {called}");
}

#[test]
fn the_api_is_available_to_the_stateful_lane_too() {
    let s = TESTING.harness();
    let env = s.execute(
        "def --env execute [data: nothing]: nothing -> record<ok: bool> {\n    grimm dbg {lane: 'stateful'}\n    { ok: true }\n}",
        "null",
    );
    assert!(!has_error(&env), "the stateful lane should carry the embedded API; got {env}");
}
