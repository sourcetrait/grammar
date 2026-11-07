//! The await tool: its suite's answer, its job's trace and its kill, and its refusals.

use std::time::{Duration, Instant};

use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_mcp::guts::{TestServer, error_kind, error_kinds};

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
    assert!(trace.contains("mcp/await/Result"), "the trace:\n{trace}");
    let first = trace.lines().next().unwrap_or_default();
    assert!(first.contains("mcp/await/Start") && first.contains("job"), "the Start comes first; the trace:\n{trace}");
    assert!(s.await_dir(&nonce).join("suite.nu").is_file(), "the suite is cached beside its trace");
}

#[test]
fn a_kill_stops_the_job_and_traces_its_stop() {
    let s = TESTING.harness();
    let env = s.await_suite(&suite(), "{nap: 30sec}", None, None);
    let nonce = text(field(&env, "nonce")).to_string();
    let listed = s.processes();
    let rows = match field(field(&listed, "result"), "processes") {
        nuval::NuVal::List(rows) => rows.clone(),
        other => panic!("a list of rows, not {other:?}"),
    };
    assert!(
        rows.iter().any(|row| text(field(row, "nonce")) == nonce && text(field(row, "tool")) == "await"),
        "the job lists in flight as await; got {rows:?}",
    );
    let killed = s.kill(&nonce);
    assert_eq!(field(&killed, "result"), &nuval::NuVal::Nothing, "{killed:?}");
    let trace = trace_with(s, &nonce, "mcp/await/Stopped");
    assert!(trace.contains("mcp/await/Stopped") && trace.contains("kill"), "the trace:\n{trace}");
    assert!(!trace.contains("mcp/await/Result"), "a stopped job sends no Result; the trace:\n{trace}");
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
