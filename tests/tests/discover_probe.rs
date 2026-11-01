//! A client's first message before `initialize`: Claude Code's `server/discover`
//! probe and its kin, each answered or refused, never a panic.

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// The revision without `initialize` that Claude Code probes for (SEP-2575).
const MODERN: &str = "2026-07-28";

/// The id Claude Code gives its first probe.
const PROBE_ID: &str = "server-discover-probe-1";

/// The per-request metadata a modern client sends with every request.
fn modern_meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": MODERN,
        "io.modelcontextprotocol/clientInfo": {"name": "grammar-system-tests", "version": "0.0.1"},
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

/// Claude Code's probe, as its client builds it.
fn discover_probe() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": PROBE_ID,
        "method": "server/discover",
        "params": {"_meta": modern_meta()},
    })
}

/// A modern request: `method` with `params` and the per-request metadata.
fn modern_request(id: u64, method: &str, mut params: Value) -> Value {
    params["_meta"] = modern_meta();
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

/// The host's own stderr.
fn host_stderr(log: &Path) -> String {
    std::fs::read_to_string(log).unwrap_or_default()
}

/// The reply to `id`, or a failure naming how the host ended.
fn reply(host: &mut Host, id: &Value, log: &Path) -> Value {
    host.try_read_value_id(id).unwrap_or_else(|| {
        let end = host
            .wait_for_exit(Duration::from_secs(5))
            .map(|status| describe_exit(&status))
            .unwrap_or_else(|| "kept running".to_string());
        panic!("no reply to {id}: the host {end}; its stderr: {}", host_stderr(log))
    })
}

/// Claude Code's probe is answered with the modern revision, and the host then
/// serves `tools/list` and an `execute` call in it with no `initialize`.
#[tested]
fn the_discover_probe_finds_the_modern_era() {
    let t = testing::test!({ .using_temp_dir() });
    let log = t.temp_dir().join("host_stderr.log");
    let mut host = Host::spawn_uninitialized(&t.temp_dir().join("scratch"), &log);

    host.send(&discover_probe());
    let answer = reply(&mut host, &json!(PROBE_ID), &log);
    let versions = answer["result"]["supportedVersions"]
        .as_array()
        .unwrap_or_else(|| panic!("a DiscoverResult; got {answer}"));
    assert!(versions.iter().any(|v| v == MODERN), "{MODERN} offered; got {answer}");
    assert!(answer["result"]["capabilities"]["tools"].is_object(), "tools offered; got {answer}");

    host.send(&modern_request(2, "tools/list", json!({})));
    let tools = reply(&mut host, &json!(2), &log);
    assert_eq!(tools["result"]["tools"].as_array().map(Vec::len), Some(4), "got {tools}");

    host.send(&modern_request(
        3,
        "tools/call",
        json!({
            "name": "execute",
            "arguments": hosted(execute_args(
                "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x + 1) } }",
                "{x: 5}",
            )),
        }),
    ));
    let called = reply(&mut host, &json!(3), &log);
    assert_eq!(envelope(&called)["result"]["out"].as_i64(), Some(6), "got {called}");
    let stderr = host_stderr(&log);
    assert!(!stderr.contains("panicked"), "no panic; the host's stderr: {stderr}");
}

/// A probe without the per-request metadata is refused with an error, and the
/// same connection then serves the legacy handshake.
#[tested]
fn a_bare_probe_is_refused_then_initialize_serves() {
    let t = testing::test!({ .using_temp_dir() });
    let log = t.temp_dir().join("host_stderr.log");
    let mut host = Host::spawn_uninitialized(&t.temp_dir().join("scratch"), &log);

    host.send(&json!({
        "jsonrpc": "2.0",
        "id": "bare-probe",
        "method": "server/discover",
        "params": {},
    }));
    let answer = reply(&mut host, &json!("bare-probe"), &log);
    assert!(answer["error"]["code"].is_i64(), "a JSON-RPC error; got {answer}");

    host.initialize();
    assert_eq!(host.tool_names().len(), 4, "the tools after the handshake");
    let stderr = host_stderr(&log);
    assert!(!stderr.contains("panicked"), "no panic; the host's stderr: {stderr}");
}

/// A first message the host cannot serve ends it with its reason on stderr
/// and a failing status, never a panic.
#[tested]
fn an_unservable_first_message_ends_the_host_cleanly() {
    let t = testing::test!({ .using_temp_dir() });
    let log = t.temp_dir().join("host_stderr.log");
    let mut host = Host::spawn_uninitialized(&t.temp_dir().join("scratch"), &log);

    host.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    let status = host
        .wait_for_exit(Duration::from_secs(10))
        .unwrap_or_else(|| panic!("the host should end; its stderr: {}", host_stderr(&log)));
    let stderr = host_stderr(&log);
    assert_eq!(status.code(), Some(1), "the host {}; its stderr: {stderr}", describe_exit(&status));
    assert!(stderr.contains("grammar: serve stdio:"), "the reason on stderr; got {stderr}");
    assert!(!stderr.contains("panicked"), "no panic; the host's stderr: {stderr}");
}
