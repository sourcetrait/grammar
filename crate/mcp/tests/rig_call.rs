
use sourcetrait_grammar_mcp::guts::{
    TestServer, error_kind, has_error, nuon, rig_block, valid_function_source, write_source,
};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary: constructing a TestServer runs
/// the namespace substrate (keypair, equip repo git config), which must not race
/// itself across parallel tests.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

#[tested]
fn call_after_commit_returns_result() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/calc");
    let _ = s.scaffold(&["sourcetrait/calc:math:double"]);
    write_source(
        &src,
        "math/double/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
    let _ = s.commit("sourcetrait/calc");
    let env = s.call("sourcetrait/calc:math:double", "{x: 7}");
    assert_eq!(env["result"]["out"].as_i64(), Some(14), "got {env}");
    let fields: Vec<&String> = env
        .as_record()
        .expect("an envelope record")
        .iter()
        .map(|(k, _)| k)
        .collect();
    assert_eq!(fields, vec!["nonce", "result"], "a call's envelope is its nonce and result; got {env}");
}

#[tested]
fn a_call_result_is_held_to_its_type() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/heldlib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(
        &src,
        "m/mod.nu",
        "export use rows\nexport use scalar\nexport use quiet\nexport use rowset\n",
    );
    // `from nuon` answers `any`, so each declared type holds at parse and only the
    // value at runtime says what came back.
    write_source(
        &src,
        "m/rows/mod.nu",
        "export def main [data: nothing]: nothing -> record<i: int> { '[[i]; [1] [2]]' | from nuon }\n",
    );
    write_source(
        &src,
        "m/scalar/mod.nu",
        "export def main [data: nothing]: nothing -> record<i: int> { '5' | from nuon }\n",
    );
    write_source(
        &src,
        "m/quiet/mod.nu",
        "export def main [data: nothing]: nothing -> nothing { }\n",
    );
    write_source(
        &src,
        "m/rowset/mod.nu",
        "export def main [data: nothing]: nothing -> table<i: int> { '[[i]; [1] [2]]' | from nuon }\n",
    );
    let committed = s.commit("sourcetrait/heldlib");
    assert!(!has_error(&committed), "got {committed}");

    let rows = s.call("sourcetrait/heldlib:m:rows", "null");
    assert_eq!(error_kind(&rows), Some("result::invalid"), "a table where a record was declared; got {rows}");
    let scalar = s.call("sourcetrait/heldlib:m:scalar", "null");
    assert_eq!(error_kind(&scalar), Some("result::invalid"), "a scalar is no record; got {scalar}");
    let quiet = s.call("sourcetrait/heldlib:m:quiet", "null");
    assert!(
        quiet.get("result").is_some_and(|result| result.is_nothing()),
        "nothing rides as null; got {quiet}",
    );
    let rowset = s.call("sourcetrait/heldlib:m:rowset", "null");
    assert_eq!(rowset["result"], nuon("[{i: 1}, {i: 2}]"), "a declared table rides as it is; got {rowset}");
}

#[tested]
fn an_undeclared_result_field_is_refused() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/leaklib");
    let _ = s.scaffold(&["sourcetrait/leaklib:m:leak"]);
    write_source(
        &src,
        "m/leak/mod.nu",
        "export def main [data: nothing]: nothing -> record<a: int> { '{a: 1, secret: 2}' | from nuon }\n",
    );
    let _ = s.commit("sourcetrait/leaklib");
    let env = s.call("sourcetrait/leaklib:m:leak", "null");
    assert_eq!(error_kind(&env), Some("result::invalid"), "got {env}");
    assert!(env.to_string().contains("result.secret: an undeclared field"), "got {env}");
}

#[tested]
fn the_timeout_reaches_a_main_that_declares_it() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/timedlib");
    let _ = s.scaffold(&["sourcetrait/timedlib:m:timed"]);
    write_source(
        &src,
        "m/timed/mod.nu",
        "export def main [data: nothing, timeout?: duration]: nothing -> record<within: bool> {\n    {within: ($timeout == 30sec)}\n}\n",
    );
    let committed = s.commit("sourcetrait/timedlib");
    assert!(!has_error(&committed), "got {committed}");
    let env = s.call_timeout("sourcetrait/timedlib:m:timed", "null", Some("30sec"));
    assert_eq!(env["result"]["within"].as_bool(), Some(true), "got {env}");
    let bad = s.call_timeout("sourcetrait/timedlib:m:timed", "null", Some("5"));
    assert_eq!(error_kind(&bad), Some("timeout::invalid"), "got {bad}");
}

#[test]
fn call_unknown_rig_errors() {
    let s = TESTING.harness();
    let env = s.call("sourcetrait/ghost:m:noop", "{noop: 0}");
    assert!(has_error(&env), "got {env}");
}

#[tested]
fn call_missing_function_errors() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let _ = s.new_rig(t.temp_dir(), "sourcetrait/partlib");
    let env = s.call("sourcetrait/partlib:m:ghost", "{noop: 0}");
    assert!(has_error(&env), "got {env}");
}

#[tested]
fn call_bad_module_path_errors() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let _ = s.new_rig(t.temp_dir(), "sourcetrait/safelib");
    for bad in [
        "sourcetrait/safelib:../etc:x",
        "sourcetrait/safelib:a/../b:x",
        "sourcetrait/safelib:/abs:x",
    ] {
        let env = s.call(bad, "{n: 0}");
        assert!(has_error(&env), "namepath {bad:?} should error; got {env}");
    }
}

#[tested]
fn call_data_is_held_to_its_type() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/strictlib");
    let _ = s.scaffold(&["sourcetrait/strictlib:m:needs_int"]);
    write_source(
        &src,
        "m/needs_int/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: $data.x }"),
    );
    let _ = s.commit("sourcetrait/strictlib");
    let mistyped = s.call("sourcetrait/strictlib:m:needs_int", "{x: five}");
    assert_eq!(error_kind(&mistyped), Some("data::invalid"), "got {mistyped}");
    let extra = s.call("sourcetrait/strictlib:m:needs_int", "{x: 1, y: 2}");
    assert_eq!(error_kind(&extra), Some("data::invalid"), "an undeclared field; got {extra}");
    let empty = s.call("sourcetrait/strictlib:m:needs_int", "{}");
    assert_eq!(error_kind(&empty), Some("data::invalid"), "a declared field missing; got {empty}");
    let not_nuon = s.call("sourcetrait/strictlib:m:needs_int", "{x: 1");
    assert_eq!(error_kind(&not_nuon), Some("data::invalid"), "got {not_nuon}");
}

#[tested]
fn inspect_returns_function_doc() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/inspectlib");
    let _ = s.scaffold(&["sourcetrait/inspectlib:math:double"]);
    write_source(
        &src,
        "math/double/mod.nu",
        "# doubles its input\n#\n# returns the doubled value\nexport def main [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x * 2) } }\n",
    );
    let _ = s.commit("sourcetrait/inspectlib");
    let env = s.inspect("sourcetrait/inspectlib:math:double");
    assert_eq!(
        env["result"].as_record().expect("a result record").len(),
        1,
        "the result is EXACTLY one field; got {env}",
    );
    let doc = &env["result"]["doc"];
    assert_eq!(
        doc["signature"].as_str(),
        Some("sourcetrait/inspectlib:math:double <x:int> <out:int> # doubles its input"),
        "the STANDALONE form carries the full namepath, both signature groups, \
         and the summary; got {env}",
    );
    assert_eq!(doc["details"].as_str(), Some("returns the doubled value"));
    assert!(
        doc["src"]
            .as_str()
            .is_some_and(|p| p.ends_with("/equip/sourcetrait/inspectlib/rig/math/double/mod.nu")),
        "src is the COMMITTED canonical mod.nu holding main; got {env}",
    );
    assert!(
        doc.get("summary").is_none(),
        "a call has no separate summary - it rides the signature; got {env}",
    );
    assert!(
        doc.get("data_type").is_none() && doc.get("result_type").is_none(),
        "the typedefs are rendered by the signature, not carried beside it; got {env}",
    );
}

#[tested]
fn inspect_rig_root_and_module() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/inspectlib2");
    write_source(&src, "mod.nu", "# the inspectlib2 rig\nexport module math\n");
    write_source(&src, "math/mod.nu", "# math helpers\nexport use double\n");
    write_source(
        &src,
        "math/double/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
    let _ = s.commit("sourcetrait/inspectlib2");
    let lib = s.inspect("sourcetrait/inspectlib2");
    assert_eq!(lib["result"]["doc"]["summary"].as_str(), Some("the inspectlib2 rig"));
    assert!(
        lib["result"]["doc"]["srcdir"]
            .as_str()
            .is_some_and(|p| p.ends_with("/equip/sourcetrait/inspectlib2/rig")),
        "a rig carries srcdir - the committed directory it lives in; got {lib}",
    );
    let m = s.inspect("sourcetrait/inspectlib2:math");
    assert_eq!(m["result"]["doc"]["summary"].as_str(), Some("math helpers"));
    assert!(
        m["result"]["doc"]["src"]
            .as_str()
            .is_some_and(|p| p.ends_with("/equip/sourcetrait/inspectlib2/rig/math/mod.nu")),
        "a module carries src - its committed mod.nu; got {m}",
    );
}

#[tested]
fn inspect_undocumented_is_empty() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/inspectlib3");
    let _ = s.scaffold(&["sourcetrait/inspectlib3:m:f"]);
    write_source(
        &src,
        "m/f/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: $data.x }"),
    );
    let _ = s.commit("sourcetrait/inspectlib3");
    let env = s.inspect("sourcetrait/inspectlib3:m:f");
    assert_eq!(
        env["result"]["doc"]["signature"].as_str(),
        Some("sourcetrait/inspectlib3:m:f <x:int> <out:int>"),
        "an undocumented call's signature carries no trailing ` # ` at all; got {env}",
    );
    assert_eq!(env["result"]["doc"]["details"].as_str(), Some(""));
}

#[test]
fn inspect_unknown_rig_errors() {
    let s = TESTING.harness();
    let env = s.inspect("sourcetrait/ghost");
    assert!(has_error(&env), "got {env}");
}

#[tested]
fn signature_groups_follow_the_contract_typedefs() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/fidelitylib");
    let _ = s.scaffold(&["sourcetrait/fidelitylib:m:shapes", "sourcetrait/fidelitylib:m:rows"]);
    write_source(
        &src,
        "m/shapes/mod.nu",
        "export def main [data: record<p: path, d: directory, c: cell-path, g: glob>]: nothing -> record<f: path> { { f: \"x\" } }\n",
    );
    write_source(
        &src,
        "m/rows/mod.nu",
        "export def main [data: nothing]: nothing -> table<n: int> { [[n]; [1]] }\n",
    );
    let committed = s.commit("sourcetrait/fidelitylib");
    assert!(!has_error(&committed), "commit should succeed; got {committed}");
    let shapes = s.inspect("sourcetrait/fidelitylib:m:shapes");
    assert_eq!(
        shapes["result"]["doc"]["signature"].as_str(),
        Some("sourcetrait/fidelitylib:m:shapes <p:path,d:directory,c:cell-path,g:glob> <f:string>"),
        "data keeps path and directory, while nushell types a returned path as a string; \
         got {shapes}",
    );
    let rows = s.inspect("sourcetrait/fidelitylib:m:rows");
    assert_eq!(
        rows["result"]["doc"]["signature"].as_str(),
        Some("sourcetrait/fidelitylib:m:rows <nothing> <table<n:int>>"),
        "a table and nothing are spelled whole; got {rows}",
    );
}

#[tested]
fn helper_file_pruned_from_info_and_not_callable() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/helperlib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use ./util.nu\nexport use real\n");
    write_source(&src, "m/util.nu", "export def helper [n: int] { $n * 2 }\n");
    write_source(
        &src,
        "m/real/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x + 1) }"),
    );
    let committed = s.commit("sourcetrait/helperlib");
    assert!(!has_error(&committed), "commit should succeed; got {committed}");

    let info = s.info();
    let signatures = info["result"]["signatures"].as_str().expect("signatures block");
    assert_eq!(
        rig_block(signatures, "helperlib"),
        " helperlib\n  m\n   real <x:int> <out:int>\n",
        "only the call-target is listed - the organizational helper file is \
         pruned from the index, so it never reaches the block",
    );

    let ok = s.call("sourcetrait/helperlib:m:real", "{x: 41}");
    assert_eq!(ok["result"]["out"].as_i64(), Some(42), "got {ok}");

    let bad = s.call("sourcetrait/helperlib:m:util", "{n: 5}");
    assert!(
        has_error(&bad),
        "an organizational helper file must NOT be callable; got {bad}",
    );
}
