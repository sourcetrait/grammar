//! The await tool: its suite's answer, its job's trace and its kill, and its refusals.

use std::time::{Duration, Instant};

use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_mcp::guts::{
    TestServer, error_kind, error_kinds, has_error, valid_function_source, write_source,
};

static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

/// The PREPARED record the suite's execute, status, and stop take.
const PREPARED: &str = "record<job: record<id: int, started: datetime>, prepared: record<nap: duration>>";

/// A suite whose job naps for prepare's duration, then answers its job's id.
fn suite() -> String {
    format!(
        "export def prepare [data: record<nap: duration>]: nothing -> record<nap: duration> {{ $data }}
export def execute [data: {PREPARED}]: nothing -> record<id: int> {{
    sleep $data.prepared.nap
    {{id: $data.job.id}}
}}
export def status [data: {PREPARED}]: nothing -> nothing {{ }}
export def stop [data: {PREPARED}]: nothing -> nothing {{ }}
"
    )
}

/// A value's field by its name, its absence a panic.
fn field<'a>(value: &'a nuval::NuVal, name: &str) -> &'a nuval::NuVal {
    match value {
        nuval::NuVal::Record(fields) => fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("no `{name}` in {value:?}")),
        other => panic!("a record, not {other:?}"),
    }
}

/// A string value's text, anything else a panic.
fn text(value: &nuval::NuVal) -> &str {
    match value {
        nuval::NuVal::String(text) => text,
        other => panic!("a string, not {other:?}"),
    }
}

/// A trace's lines, each a record of its model, its time, and its data.
fn trace_lines(trace: &str) -> Vec<nuval::NuVal> {
    trace
        .lines()
        .map(|line| nuval::NuVal::from_nuon(line).unwrap_or_else(|e| panic!("a trace line is NUON: {e}\n{line}")))
        .collect()
}

/// A record's field names, in order.
fn names(value: &nuval::NuVal) -> Vec<&str> {
    match value {
        nuval::NuVal::Record(fields) => fields.iter().map(|(key, _)| key.as_str()).collect(),
        other => panic!("a record, not {other:?}"),
    }
}

/// The job's trace once a line of `model` lands in it, or after ten seconds.
fn trace_with(s: &TestServer, nonce: &str, model: &str) -> String {
    let file = s.await_dir(nonce).join("trace.nuonl");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let trace = std::fs::read_to_string(&file).unwrap_or_default();
        if trace.contains(model) || Instant::now() >= deadline {
            return trace;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn await_answers_the_prepared_record_and_traces_the_result() {
    let s = TESTING.harness();
    let env = s.await_suite(&suite(), "{nap: 10ms}", None, None);
    let result = field(&env, "result");
    assert_eq!(field(result, "prepared"), &nuval::NuVal::Record(vec![("nap".into(), nuval::NuVal::Duration(10_000_000))]));
    let nonce = text(field(&env, "nonce")).to_string();
    let trace = trace_with(s, &nonce, "mcp/await/Result");
    let lines = trace_lines(&trace);
    let [line] = lines.as_slice() else {
        panic!("the trace is the Result alone; the trace:\n{trace}");
    };
    assert_eq!(text(field(line, "model")), "mcp/await/Result", "the trace:\n{trace}");
    let data = field(line, "data");
    assert_eq!(names(data), vec!["await", "result"], "the trace:\n{trace}");
    assert_eq!(text(field(data, "await")), nonce, "the trace:\n{trace}");
    assert!(s.await_dir(&nonce).join("suite.nu").is_file(), "the suite is cached beside its trace");
}

#[test]
fn a_stop_by_hand_stops_the_job_and_traces_its_stop() {
    let s = TESTING.harness();
    let env = s.await_suite(&suite(), "{nap: 30sec}", None, None);
    let nonce = text(field(&env, "nonce")).to_string();
    let listed = s.awaits();
    let rows = match field(&listed, "result") {
        nuval::NuVal::List(rows) => rows.clone(),
        other => panic!("a table of rows, not {other:?}"),
    };
    let row = rows
        .iter()
        .find(|row| text(field(row, "await")) == nonce)
        .unwrap_or_else(|| panic!("the job lists by its await; got {rows:?}"));
    assert_eq!(names(row), vec!["await", "job", "started"], "{row:?}");
    let status = s.await_status(&nonce);
    let answered = field(&status, "result");
    assert_eq!(names(answered), vec!["await", "status"], "{status:?}");
    assert_eq!(field(answered, "status"), &nuval::NuVal::Nothing, "status's value, null here; {status:?}");
    let stopped = s.await_stop(&nonce);
    assert_eq!(field(&stopped, "result"), &nuval::NuVal::Nothing, "{stopped:?}");
    let trace = trace_with(s, &nonce, "mcp/await/Stop");
    let lines = trace_lines(&trace);
    let stop = lines
        .iter()
        .find(|line| text(field(line, "model")) == "mcp/await/Stop")
        .unwrap_or_else(|| panic!("a Stop; the trace:\n{trace}"));
    let data = field(stop, "data");
    assert_eq!(names(data), vec!["await", "reason", "result"], "the trace:\n{trace}");
    assert_eq!(text(field(data, "reason")), "stop", "the trace:\n{trace}");
    assert!(
        lines.iter().all(|line| text(field(line, "model")) != "mcp/await/Result"),
        "a stopped job sends no Result; the trace:\n{trace}",
    );
}

#[tested]
fn await_rig_starts_a_suite_from_a_rigs_module_directory() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/awaitrig");
    write_source(&src, "mod.nu", "export module long\n");
    write_source(&src, "long/mod.nu", "export module nap\nexport use calc\n");
    write_source(&src, "long/nap/mod.nu", &suite());
    write_source(&src, "long/calc/mod.nu", &valid_function_source("x: int", "y: int", "{ y: $data.x }"));
    let committed = s.commit("sourcetrait/awaitrig");
    assert!(!has_error(&committed), "the rig commits: {committed:?}");
    let env = s.interface("interface await rig equip/sourcetrait/awaitrig/rig/long/nap {nap: 10ms}");
    let result = field(&env, "result");
    assert_eq!(field(result, "prepared"), &nuval::NuVal::Record(vec![("nap".into(), nuval::NuVal::Duration(10_000_000))]));
    let nonce = text(field(&env, "nonce")).to_string();
    let trace = trace_with(s, &nonce, "mcp/await/Result");
    let lines = trace_lines(&trace);
    let result = lines
        .iter()
        .find(|line| text(field(line, "model")) == "mcp/await/Result")
        .unwrap_or_else(|| panic!("the job's Result under the interface call's nonce; the trace:\n{trace}"));
    assert_eq!(text(field(field(result, "data"), "await")), nonce, "the trace:\n{trace}");
    let off_form = s.interface("await rig equip/sourcetrait/awaitrig/lib/nap {nap: 10ms}");
    assert_eq!(error_kind(&off_form), Some("interface::invalid"), "{off_form:?}");
    let climbing = s.interface("await rig equip/sourcetrait/awaitrig/rig/../rig/long/nap {nap: 10ms}");
    assert_eq!(error_kind(&climbing), Some("interface::invalid"), "{climbing:?}");
    let missing = s.interface("await rig equip/sourcetrait/awaitrig/rig/long/gone {nap: 10ms}");
    assert_eq!(error_kind(&missing), Some("module::invalid"), "{missing:?}");
    let call = s.interface("await rig equip/sourcetrait/awaitrig/rig/long/calc {x: 1}");
    let kinds = error_kinds(&call);
    assert!(!kinds.is_empty() && kinds.iter().all(|kind| kind == "module::invalid"), "a call is no suite: {call:?}");
}

#[tested]
fn await_imports_a_suite_by_its_absolute_path() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let dir = t.temp_dir().join("nap");
    write_source(&dir, "mod.nu", &suite());
    let env = s.await_suite(&format!("export use {} *", dir.display()), "{nap: 10ms}", None, None);
    assert!(!has_error(&env), "an absolute `export use` path is the parser's, never the lint's: {env:?}");
    let result = field(&env, "result");
    assert_eq!(field(result, "prepared"), &nuval::NuVal::Record(vec![("nap".into(), nuval::NuVal::Duration(10_000_000))]));
}

#[test]
fn await_refuses_a_suite_and_parameters_off_the_contract() {
    let s = TESTING.harness();
    let env = s.await_suite("export def prepare [data: nothing]: nothing -> nothing { }", "null", None, None);
    assert_eq!(error_kinds(&env), vec!["module::invalid".to_string(); 3], "{env:?}");
    let env = s.await_suite(&suite(), "{nap: ", None, None);
    assert_eq!(error_kind(&env), Some("data::invalid"), "{env:?}");
    let env = s.await_suite(&suite(), "{nap: 10ms}", Some("{prepare: "), None);
    assert_eq!(error_kind(&env), Some("timeout::invalid"), "{env:?}");
    let env = s.await_suite(&suite(), "{nap: 10ms}", None, Some("[1sec]"));
    assert_eq!(error_kind(&env), Some("heartbeats::invalid"), "{env:?}");
}
