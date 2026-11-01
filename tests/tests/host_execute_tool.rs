
use serde_json::json;
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// initialize -> tools/list (exactly 4) -> an execute() round-trip. The rmcp stdio
/// transport + handshake are the system surface here.
#[tested]
fn tools_list_and_execute_round_trip() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());

    let names = host.tool_names();
    assert_eq!(names.len(), 4, "expected 4 tools; got {names:?}");
    for expected in ["execute", "rerun", "call", "interface"] {
        assert!(
            names.contains(&expected.to_string()),
            "missing `{expected}` in {names:?}",
        );
    }

    let resp = host.execute(
        "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x + 1) } }",
        "{x: 5}",
    );
    let env = envelope(&resp);
    let nonce = env["nonce"].as_str().expect("envelope nonce");
    assert!(
        !nonce.is_empty() && nonce.chars().all(|c| c.is_ascii_alphanumeric()),
        "nonce should be base62; got {nonce:?}",
    );
    assert_eq!(env["result"], nuon("{out: 6}"), "got {env}");
}

/// Every tool requires `host`: its schema lists it, any alias but `local` is
/// refused as `host::unknown` before the tool does anything, and a call naming
/// no host fails to deserialize at rmcp's boundary, which answers an `isError`
/// result carrying its own text rather than an envelope.
#[tested]
fn every_tool_names_its_host() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());

    for tool in host.tools() {
        let required = tool["inputSchema"]["required"]
            .as_array()
            .unwrap_or_else(|| panic!("required parameters; got {tool}"));
        assert!(required.contains(&json!("host")), "{tool}");
    }

    for (tool, mut args) in [
        (
            "execute",
            execute_args("def execute [data: nothing]: nothing -> record<ok: bool> { {ok: true} }", "null"),
        ),
        ("rerun", json!({"nonce": "NoSuchNonce1", "data": "null"})),
        ("call", json!({"namepath": "sourcetrait/ghost:m:noop", "data": "null"})),
        ("interface", json!({"command": "interface info"})),
    ] {
        args["host"] = json!("elsewhere");
        let resp = host.call(tool, args);
        assert_eq!(envelope_error_kind(&resp).as_deref(), Some("host::unknown"), "{tool}: {resp}");
    }

    let id = host.request_as_given("interface", json!({"command": "interface info"}));
    let resp = host.read_id(id);
    assert_eq!(resp["result"]["isError"], json!(true), "refused at the boundary; got {resp}");
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("missing field `host`")),
        "rmcp names the missing field; got {resp}",
    );
}
