//! Template synthesis and parse cleanliness, through the `guts` surface.
//!
//! Integration rather than unit: `parses_clean` builds a full-shell
//! `ParseEngine`, its plugin registry read off disk, so it rides the resource
//! caveat.

use std::path::Path;

use sourcetrait_grammar_mcp::guts::{build_call_source, build_execute_source, parses_clean};

const DEFINITION: &str =
    "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x + 1) } }";

#[test]
fn nu_source_is_the_definition_then_execute_on_the_data() {
    let got = build_execute_source(DEFINITION, "{x: 5}", None);
    assert_eq!(got, format!("{DEFINITION}\nexecute {{x: 5}}\n"));
    assert!(parses_clean(&got), "a nu source must parse clean:\n{got}");
}

#[test]
fn nu_source_passes_a_null_for_nothing() {
    let definition = "def execute [data: nothing]: nothing -> record<out: int> { { out: 0 } }";
    let got = build_execute_source(definition, "null", None);
    assert_eq!(got, format!("{definition}\nexecute null\n"));
    assert!(parses_clean(&got), "a void nu source must parse clean:\n{got}");
}

#[test]
fn nu_source_passes_the_timeout_to_a_definition_that_takes_it() {
    let definition = "def execute [data: nothing, timeout?: duration]: nothing -> record<out: int> { { out: 0 } }";
    let got = build_execute_source(definition, "null", Some("30sec"));
    assert_eq!(got, format!("{definition}\nexecute null 30sec\n"));
    assert!(parses_clean(&got), "a nu source with a timeout must parse clean:\n{got}");
}

/// A definition is spliced verbatim, so a multi-line string literal survives.
#[test]
fn nu_source_keeps_a_multi_line_definition_verbatim() {
    let definition = "def execute [data: record<x: int>]: nothing -> record<out: int> {\n    let y = ($data.x * 2)\n{ out: $y }\n}";
    let got = build_execute_source(definition, "{x: 3}", None);
    assert_eq!(got, format!("{definition}\nexecute {{x: 3}}\n"));
    assert!(parses_clean(&got), "a multi-line nu source must parse clean:\n{got}");
}

/// The equip root the call sources below are built against.
fn root() -> &'static Path {
    Path::new("/ns/equip")
}

/// A call source, its build held to succeed.
fn call_source(
    rig: &str,
    module_path: &str,
    name: &str,
    data_literal: &str,
    timeout_literal: Option<&str>,
) -> String {
    build_call_source(root(), rig, module_path, name, data_literal, timeout_literal)
        .expect("a call source builds")
}

/// The call's target as the build joins it, quoted as the string the source carries:
/// `"/ns/equip/acme/util/rig/net/ping"` on Unix, each join a backslash on Windows.
fn target(root: &Path, rig: &str, module_path: &str, name: &str) -> String {
    let path = root.join(rig).join("rig").join(module_path).join(name);
    format!("{:?}", path.to_str().expect("a UTF-8 path"))
}

#[test]
fn call_source_takes_the_data_literal() {
    let got = call_source("sourcetrait/calc", "math", "double", "{x: 6}", None);
    let expected = format!(
        "overlay use --prefix {} as __call\n__call {{x: 6}}\n",
        target(root(), "sourcetrait/calc", "math", "double"),
    );
    assert_eq!(got, expected);
}

#[test]
fn call_source_nested_module_path() {
    let got = call_source("sourcetrait/calc", "math/trig", "sin", "{x: 1}", None);
    let expected = format!(
        "overlay use --prefix {} as __call\n__call {{x: 1}}\n",
        target(root(), "sourcetrait/calc", "math/trig", "sin"),
    );
    assert_eq!(got, expected);
}

#[test]
fn call_source_keeps_an_empty_record_and_passes_null_as_given() {
    let ping = target(root(), "acme/util", "net", "ping");
    let empty = call_source("acme/util", "net", "ping", "{}", None);
    assert_eq!(empty, format!("overlay use --prefix {ping} as __call\n__call {{}}\n"));
    let null = call_source("acme/util", "net", "ping", "null", None);
    assert_eq!(null, format!("overlay use --prefix {ping} as __call\n__call null\n"));
}

#[test]
fn call_source_passes_the_timeout_to_a_main_that_takes_it() {
    let got = call_source("acme/util", "net", "ping", "null", Some("30sec"));
    let ping = target(root(), "acme/util", "net", "ping");
    assert_eq!(got, format!("overlay use --prefix {ping} as __call\n__call null 30sec\n"));
}

#[test]
fn call_source_keyword_named_target_is_aliased() {
    let got = call_source("sourcetrait/demo", "almost/rpg", "run", "{x: 6}", None);
    let expected = format!(
        "overlay use --prefix {} as __call\n__call {{x: 6}}\n",
        target(root(), "sourcetrait/demo", "almost/rpg", "run"),
    );
    assert_eq!(got, expected);
    assert!(
        !expected.contains("\nrun "),
        "the drive must never be the target's own name",
    );
}

/// A root holding a space still names one path, the target a string literal.
#[test]
fn call_source_quotes_a_root_holding_a_space() {
    let root = Path::new("/my ns/equip");
    let got = build_call_source(root, "acme/util", "net", "ping", "{}", None)
        .expect("a call source builds");
    let ping = target(root, "acme/util", "net", "ping");
    assert_eq!(got, format!("overlay use --prefix {ping} as __call\n__call {{}}\n"));
}
