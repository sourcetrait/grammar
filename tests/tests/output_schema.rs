//! No tool declares an output schema, and every result is one NUON text content
//! led by its nonce, a failure's as much as a success's, never structured content.

use serde_json::{Value, json};
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

#[tested]
fn no_tool_declares_an_output_schema() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());
    let declaring: Vec<String> = host
        .tools()
        .into_iter()
        .filter(|tool| tool.get("outputSchema").is_some())
        .map(|tool| tool["name"].as_str().unwrap_or_default().to_string())
        .collect();
    assert!(declaring.is_empty(), "tools declaring an output schema: {declaring:?}");
}

/// The one text content of a response, asserted the only content.
fn the_text(resp: &Value) -> String {
    let result = &resp["result"];
    assert!(result.get("structuredContent").is_none(), "no structured content; got {resp}");
    assert_ne!(result["isError"], json!(true), "a failure rides a plain result; got {resp}");
    let content = result["content"].as_array().unwrap_or_else(|| panic!("content; got {resp}"));
    assert_eq!(content.len(), 1, "exactly one content; got {resp}");
    assert_eq!(content[0]["type"], "text", "a text content; got {resp}");
    content[0]["text"].as_str().expect("its text").to_string()
}

#[tested]
fn every_result_is_one_nuon_text_led_by_its_nonce() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());

    let ok = host.execute(
        "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x + 1) } }",
        "{x: 5}",
    );
    let text = the_text(&ok);
    assert!(text.starts_with("{nonce: "), "the nonce leads; got {text}");
    assert_eq!(nuon(&text)["result"], nuon("{out: 6}"), "got {text}");

    let failed = host.execute(
        "def execute [data: nothing]: nothing -> record<x: string> { { x: '/tmp/x' } }",
        "null",
    );
    let text = the_text(&failed);
    assert!(text.starts_with("{nonce: "), "the nonce leads a failure too; got {text}");
    assert_eq!(
        envelope_error_kind(&failed).as_deref(),
        Some("lint::hardcoded_variable"),
        "got {text}",
    );

    let listed = host.interface("interface purview list");
    let text = the_text(&listed);
    assert!(text.starts_with("{nonce: "), "an interface command's too; got {text}");

    let refused = host.interface("interface nosuch");
    let text = the_text(&refused);
    assert!(text.starts_with("{nonce: "), "and an interface refusal's; got {text}");
}
