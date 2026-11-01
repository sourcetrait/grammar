
use sourcetrait_grammar_mcp::guts::{
    NuVal, TestServer, equip_source, error_kind, error_kinds, error_messages, error_text,
    has_error, valid_function_source, write_source,
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
fn commit_accepts_path_self_call_target() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/selflib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use whereami\n");
    write_source(
        &src,
        "m/whereami/mod.nu",
        "export def main [data: record<noop: int>]: nothing -> record<here: string> {\n    const SELF = (path self)\n    { here: $SELF }\n}\n",
    );
    let env = s.commit("sourcetrait/selflib");
    assert!(
        !has_error(&env),
        "a call-target using `path self` should commit; got {env}"
    );
}

#[tested]
fn commit_accepts_path_self_in_mod_nu_const() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/modselflib");
    write_source(
        &src,
        "mod.nu",
        "export const HERE = (path self)\nexport module m\n",
    );
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
    let env = s.commit("sourcetrait/modselflib");
    assert!(
        !has_error(&env),
        "a mod.nu `export const` using `path self` should commit; got {env}"
    );
}

#[tested]
fn commit_happy_path_writes_repo_and_meta() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/happylib");
    write_source(&src, "mod.nu", "export module math\n");
    write_source(&src, "math/mod.nu", "export use double\n");
    write_source(
        &src,
        "math/double/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );

    let env = s.commit("sourcetrait/happylib");
    assert!(!has_error(&env), "commit should succeed; got {env}");

    let lib = s.rig_dir("sourcetrait/happylib");
    assert!(lib.join("mod.nu").exists());
    assert!(lib.join("math").join("mod.nu").exists());
    assert!(lib.join("math").join("double").join("mod.nu").exists());

    assert!(
        lib.join(".meta/rig.nuon").exists(),
        "the index is NUON on disk, not JSON",
    );
    let meta = s.rig_index("sourcetrait/happylib");
    assert_eq!(meta["source_path"].as_str(), Some(src.to_str().unwrap()));
    assert!(
        meta.get("kind").is_none(),
        "meta should not carry a kind field; got {meta}",
    );
}

#[tested]
fn commit_rejects_mod_nu_with_syntax_error() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/badmodlib");
    write_source(&src, "mod.nu", "export module foo\nexport\n");
    write_source(&src, "foo/mod.nu", "");
    let env = s.commit("sourcetrait/badmodlib");
    assert!(has_error(&env), "syntax-error mod.nu should reject; got {env}");
    let msg = error_text(&env);
    assert!(
        msg.contains("parse error"),
        "expected parse-error violation in mod.nu; got {msg:?}",
    );
}

#[tested]
fn commit_rejects_mod_nu_referencing_missing_file() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/missingreflib");
    write_source(&src, "mod.nu", "export use ./does_not_exist.nu\n");
    let env = s.commit("sourcetrait/missingreflib");
    assert!(has_error(&env), "missing-ref mod.nu should reject; got {env}");
    let msg = error_text(&env);
    assert!(
        msg.contains("ModuleNotFound") || msg.contains("does_not_exist"),
        "expected ModuleNotFound for missing ref; got {msg:?}",
    );
}

#[tested]
fn commit_accepts_multiline_def_signature() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/multilinelib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        "export def main [\n    data: record<x: int>\n]: nothing -> record<out: int> {\n    { out: ($data.x * 2) }\n}\n",
    );
    let env = s.commit("sourcetrait/multilinelib");
    assert!(!has_error(&env), "multi-line signature should pass; got {env}");
}

#[tested]
fn commit_rejects_function_with_syntax_error() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/syntaxlib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export module broken\n");
    write_source(
        &src,
        "m/broken/mod.nu",
        "export def main [data: record<x: int>]: nothing -> record<out: int> {\n    { out: ($data.x * 2) \n",
    );
    let env = s.commit("sourcetrait/syntaxlib");
    assert!(has_error(&env));
    let msg = error_text(&env);
    assert!(
        msg.contains("parse error"),
        "expected parse error violation; got {msg:?}",
    );
}

#[tested]
fn commit_rejects_main_without_output_type() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/badlib1");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        "export def main [data: record<x: int>] { { out: $data.x } }\n",
    );

    let env = s.commit("sourcetrait/badlib1");
    assert!(has_error(&env));
    let msg = error_text(&env);
    assert!(
        msg.contains("rig::contract") && msg.contains("infix signature"),
        "expected the contract's missing-signature violation; got {msg:?}",
    );
}

#[tested]
fn commit_accepts_call_target_with_helper_export() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/helperexportlib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        "export def helper [n: int] { $n * 2 }\nexport def main [data: record<x: int>]: nothing -> record<out: int> { { out: $data.x } }\n",
    );

    let env = s.commit("sourcetrait/helperexportlib");
    assert!(
        !has_error(&env),
        "a call-target may export helpers beside main; got {env}"
    );
    let called = s.call("sourcetrait/helperexportlib:m:thing", "{x: 5}");
    assert_eq!(
        called["result"]["out"].as_i64(),
        Some(5),
        "call() should run main; got {called}",
    );
}

#[tested]
fn commit_rejects_main_empty_record_output() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/badlib3");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        "export def main [data: record<x: int>]: nothing -> record<> { {} }\n",
    );

    let env = s.commit("sourcetrait/badlib3");
    assert!(has_error(&env));
    assert_eq!(error_kind(&env), Some("rig::contract"), "got {env}");
    let msg = error_text(&env);
    assert!(
        msg.contains("the return type") && msg.contains("declare the fields"),
        "expected the open return record refused; got {msg:?}",
    );
}

#[tested]
fn commit_accepts_mod_nu_with_inline_const() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/constmodlib");
    write_source(&src, "mod.nu", "export module sub\nconst X = 42\n");
    write_source(&src, "sub/mod.nu", "");
    let env = s.commit("sourcetrait/constmodlib");
    assert!(
        !has_error(&env),
        "a private const in mod.nu is allowed; got {env}"
    );
}

#[tested]
fn commit_accepts_mod_nu_with_inline_alias() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/aliasmodlib");
    write_source(&src, "mod.nu", "export module sub\nalias foo = ls\n");
    write_source(&src, "sub/mod.nu", "");
    let env = s.commit("sourcetrait/aliasmodlib");
    assert!(
        !has_error(&env),
        "a private alias in mod.nu is allowed; got {env}"
    );
}

#[tested]
fn commit_rejects_mod_nu_with_let() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/letmodlib");
    write_source(&src, "mod.nu", "let x = 5\n");
    let env = s.commit("sourcetrait/letmodlib");
    assert!(has_error(&env));
    let msg = error_text(&env);
    assert!(
        msg.contains("parse error") || msg.contains("Expected"),
        "expected parse error for let in mod.nu; got {msg:?}",
    );
}

#[tested]
fn commit_accepts_mod_nu_with_only_comments() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/commentedmodlib");
    write_source(&src, "mod.nu", "# this rig is empty\n# more comment\n");
    let env = s.commit("sourcetrait/commentedmodlib");
    assert!(
        !has_error(&env),
        "empty mod.nu with comments should accept; got {env}"
    );
}

#[tested]
fn commit_accepts_mod_nu_with_inline_def() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/badlib4");
    write_source(&src, "mod.nu", "export module sub\ndef helper [] { 99 }\n");
    write_source(&src, "sub/mod.nu", "");

    let env = s.commit("sourcetrait/badlib4");
    assert!(
        !has_error(&env),
        "a private def in mod.nu is allowed; got {env}"
    );
}

#[tested]
fn commit_aggregates_multiple_violations() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/badlib5");
    write_source(&src, "mod.nu", "export module a\nextern noise []\n");
    write_source(&src, "a/mod.nu", "export use skel\nexport use noout\n");
    write_source(
        &src,
        "a/skel/mod.nu",
        "export def main [data: record<>]: nothing -> record<out: int> { { out: 1 } }\n",
    );
    write_source(
        &src,
        "a/noout/mod.nu",
        "export def main [data: record<x: int>] { { out: $data.x } }\n",
    );

    let env = s.commit("sourcetrait/badlib5");
    let messages = error_messages(&env);
    assert!(
        messages.iter().any(|m| m.contains("mod.nu may only contain")),
        "got {messages:?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("`data`") && m.contains("declare the fields")),
        "got {messages:?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("infix signature")),
        "got {messages:?}"
    );
}

#[tested]
fn commit_caps_structural_violations() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/caplib");
    write_source(
        &src,
        "mod.nu",
        "export module a\nextern e1 []\nextern e2 []\nextern e3 []\nextern e4 []\n",
    );
    write_source(&src, "a/mod.nu", "");
    let env = s.commit("sourcetrait/caplib");
    let errors = env["error"].as_list().expect("an error table");
    assert_eq!(errors.len(), 3, "errors should cap at 3; got {errors:?}");
}

#[tested]
fn commit_rejects_root_call_target() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/rootfnlib");
    write_source(&src, "mod.nu", "export use thing\n");
    write_source(
        &src,
        "thing/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
    let env = s.commit("sourcetrait/rootfnlib");
    assert!(
        error_kinds(&env).iter().any(|k| k == "rig::root_function"),
        "expected rig::root_function; got {:?}",
        error_kinds(&env),
    );
    let messages = error_messages(&env);
    assert!(
        messages.iter().any(|m| m.contains("rig root") && m.contains("module")),
        "got {messages:?}"
    );
}

#[tested]
fn commit_succeeds_then_check_warns_long_summary() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/doclib");
    let long = "x".repeat(81);
    write_source(&src, "mod.nu", &format!("# {long}\nexport module m\n"));
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: $data.x }"),
    );
    let committed = s.commit("sourcetrait/doclib");
    assert!(
        !has_error(&committed),
        "an over-long summary is advisory; commit should succeed; got {committed}"
    );
    assert!(
        committed["result"]["warnings"]
            .as_list()
            .expect("the commit's warnings")
            .iter()
            .any(|warning| warning["kind"] == "lint::summary_length"),
        "the commit's own result carries the warning; got {committed}",
    );

    let check = s.equip("check", src.parent().unwrap().to_str().unwrap());
    let summary = &check["result"]["summary"];
    assert_eq!(
        summary["ok"].as_bool(),
        Some(true),
        "warnings don't fail check; got {check}"
    );
    let warn_kinds: Vec<&str> = summary["warnings"]
        .as_list()
        .expect("warnings array")
        .iter()
        .filter_map(|w| w["kind"].as_str())
        .collect();
    assert!(
        warn_kinds.contains(&"lint::summary_length"),
        "expected lint::summary_length warning; got {warn_kinds:?}"
    );
}

#[tested]
fn commit_accepts_short_summary() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/okdoclib");
    write_source(
        &src,
        "mod.nu",
        "# doubles its input\n# the math double helper module\nexport module m\n",
    );
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
    let env = s.commit("sourcetrait/okdoclib");
    assert!(
        !has_error(&env),
        "documented rig should commit; got {env}"
    );
}

#[tested]
fn equip_new_reestablish_duplicate_errors() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/duplib");
    let r1 = s.equip("new", eq.to_str().unwrap());
    assert!(!has_error(&r1), "first equip(new) should succeed; got {r1}");
    let r2 = s.equip("new", eq.to_str().unwrap());
    assert!(has_error(&r2), "duplicate establish should error; got {r2}");
    assert_eq!(
        error_kind(&r2),
        Some("equip::already_registered"),
        "got {r2}"
    );
}

#[tested]
fn commit_picks_up_mutated_source() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/livelib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
    let _ = s.commit("sourcetrait/livelib");
    write_source(
        &src,
        "m/thing/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x + 1000) }"),
    );
    let env = s.commit("sourcetrait/livelib");
    assert!(!has_error(&env), "re-commit should succeed; got {env}");
    let committed = std::fs::read_to_string(
        s.rig_dir("sourcetrait/livelib").join("m").join("thing").join("mod.nu"),
    )
    .unwrap();
    assert!(
        committed.contains("+ 1000"),
        "committed body should reflect mutated source; got {committed:?}",
    );
}

#[test]
fn commit_unknown_rig_errors() {
    let s = TESTING.harness();
    let env = s.commit("sourcetrait/ghost");
    assert!(has_error(&env), "got {env}");
}

#[tested]
fn commit_validates_by_name_cross_rig_use() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();

    let base = s.new_rig(t.temp_dir(), "sourcetrait/baselib");
    write_source(&base, "mod.nu", "export module m\n");
    write_source(&base, "m/mod.nu", "export use double\n");
    write_source(
        &base,
        "m/double/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
    let committed_base = s.commit("sourcetrait/baselib");
    assert!(
        !has_error(&committed_base),
        "baselib commit should succeed; got {committed_base}"
    );

    let consumer = s.new_rig(t.temp_dir(), "sourcetrait/consumer");
    write_source(&consumer, "mod.nu", "export module app\n");
    write_source(&consumer, "app/mod.nu", "export use compute\n");
    write_source(
        &consumer,
        "app/compute/mod.nu",
        "use equip/sourcetrait/baselib/rig/m\nexport def main [data: record<x: int>]: nothing -> record<out: int> {\n    m double {x: $data.x}\n}\n",
    );
    let committed = s.commit("sourcetrait/consumer");
    assert!(
        !has_error(&committed),
        "a rig using a committed sibling by name should commit; got {committed}"
    );

    let called = s.call("sourcetrait/consumer:app:compute", "{x: 5}");
    assert_eq!(
        called["result"]["out"].as_i64(),
        Some(10),
        "cross-rig call-target should resolve + run; got {called}",
    );
}

#[tested]
fn commit_and_call_resolves_authored_self_ref() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/selfreflib");
    write_source(&src, "mod.nu", "export module base\nexport module top\n");
    write_source(&src, "base/mod.nu", "export def val []: nothing -> int { 21 }\n");
    write_source(&src, "top/mod.nu", "export use double\n");
    write_source(
        &src,
        "top/double/mod.nu",
        "use equip/sourcetrait/selfreflib/rig/base\nexport def main [data: nothing]: nothing -> record<out: int> {\n    { out: ((base val) * 2) }\n}\n",
    );
    let committed = s.commit("sourcetrait/selfreflib");
    assert!(
        !has_error(&committed),
        "a rig with an authored self-ref target must validate at commit; got {committed}"
    );
    let called = s.call("sourcetrait/selfreflib:top:double", "null");
    assert_eq!(
        called["result"]["out"].as_i64(),
        Some(42),
        "the authored self-ref must resolve + run at serve; got {called}"
    );
}

#[tested]
fn commit_accepts_organizational_file() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/orglib");
    write_source(&src, "mod.nu", "export use ./util.nu\n");
    write_source(
        &src,
        "util.nu",
        "export const LIMIT = 10\nexport def helper [n: int] { $n * 2 }\n",
    );
    let env = s.commit("sourcetrait/orglib");
    assert!(
        !has_error(&env),
        "organizational file should commit; got {env}"
    );
}

#[tested]
fn commit_accepts_mod_nu_with_export_const_and_def() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/modutillib");
    write_source(
        &src,
        "mod.nu",
        "export const VERSION = 1\nexport def shared [] { 42 }\nexport module m\n",
    );
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
    let env = s.commit("sourcetrait/modutillib");
    assert!(
        !has_error(&env),
        "mod.nu with export const/def should commit; got {env}"
    );
}

#[tested]
fn commit_rejects_empty_record_skeleton() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/skellib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        "export def main [data: record<>]: nothing -> record<n: int> { { n: 1 } }\n",
    );
    let env = s.commit("sourcetrait/skellib");
    assert!(has_error(&env));
    assert_eq!(error_kind(&env), Some("rig::contract"), "got {env}");
    let msg = error_text(&env);
    assert!(
        msg.contains("`data`") && msg.contains("declare the fields"),
        "expected the open data record refused; got {msg:?}",
    );
}

#[tested]
fn commit_rejects_private_def_named_reserved() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/pdeflib");
    write_source(&src, "mod.nu", "export use ./util.nu\n");
    write_source(&src, "util.nu", "export const LIMIT = 5\ndef main [] { 1 }\n");
    let env = s.commit("sourcetrait/pdeflib");
    assert!(has_error(&env));
    let msg = error_text(&env);
    assert!(
        msg.contains("reserved"),
        "expected reserved-term violation; got {msg:?}"
    );
}

#[tested]
fn commit_rejects_module_named_reserved() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/modreslib");
    std::fs::create_dir_all(src.join("main")).unwrap();
    write_source(&src, "mod.nu", "export module main\n");
    write_source(&src, "main/mod.nu", "");
    let env = s.commit("sourcetrait/modreslib");
    assert!(has_error(&env));
    let msg = error_text(&env);
    assert!(
        msg.contains("reserved"),
        "expected reserved-name violation; got {msg:?}"
    );
}

#[tested]
fn commit_rejects_const_named_reserved() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/constreslib");
    write_source(&src, "mod.nu", "export const main = 5\nexport module m\n");
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
    let env = s.commit("sourcetrait/constreslib");
    assert!(has_error(&env));
    let msg = error_text(&env);
    assert!(
        msg.contains("reserved"),
        "expected reserved-const violation; got {msg:?}"
    );
}

#[tested]
fn commit_rejects_record_key_reserved() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/rkeylib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        "export def main [data: record<x: int>]: nothing -> record<out: int> { { out: $data.x, main: 1 } }\n",
    );
    let env = s.commit("sourcetrait/rkeylib");
    assert!(has_error(&env));
    let msg = error_text(&env);
    assert!(
        msg.contains("reserved"),
        "expected reserved record-key violation; got {msg:?}"
    );
}

#[tested]
fn commit_rejects_cellpath_member_reserved() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/cpathlib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        "export def main [data: record<x: int>]: nothing -> record<out: int> { { out: $data.main } }\n",
    );
    let env = s.commit("sourcetrait/cpathlib");
    assert!(has_error(&env));
    let msg = error_text(&env);
    assert!(
        msg.contains("reserved"),
        "expected reserved cell-path violation; got {msg:?}"
    );
}

#[tested]
fn commit_accepts_reserved_as_quoted_string_value() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/strvallib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        "export def main [data: record<x: int>]: nothing -> record<out: int> { let note = \"main\"; { out: $data.x } }\n",
    );
    let env = s.commit("sourcetrait/strvallib");
    assert!(
        !has_error(&env),
        "quoted string value should not trip the ban; got {env}"
    );
}

#[tested]
fn equip_new_and_scaffold_function() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/scaffolded");
    let src = eq.join("rig");
    let r1 = s.equip("new", eq.to_str().unwrap());
    assert!(!has_error(&r1), "establish should succeed; got {r1}");
    let meta = s.rig_index("sourcetrait/scaffolded");
    assert_eq!(
        meta["source_path"].as_str(),
        Some(src.to_str().unwrap()),
        "meta should record source_path; got {meta}",
    );
    assert!(
        src.join("mod.nu").exists(),
        "source root mod.nu should be seeded"
    );

    let r2 = s.scaffold(&["sourcetrait/scaffolded:math:double"]);
    assert!(!has_error(&r2), "scaffold function should succeed; got {r2}");
    let fn_src = std::fs::read_to_string(src.join("math").join("double").join("mod.nu")).unwrap();
    assert!(
        fn_src.contains("export def main [data: record<>]: nothing -> record<>"),
        "skeleton missing the single infix-signatured main; got {fn_src:?}"
    );
    assert!(
        !fn_src.contains("export def call") && !fn_src.contains("export def resolve"),
        "skeleton should be 1-def main only; got {fn_src:?}"
    );
    let math_mod = std::fs::read_to_string(src.join("math").join("mod.nu")).unwrap();
    assert!(
        math_mod.contains("export module double"),
        "math mod.nu should DECLARE the call as a submodule; got {math_mod:?}",
    );
    assert!(
        math_mod.contains("export use double"),
        "math mod.nu should RE-EXPORT the call (0.114 no longer implicitly imports \
         submodules, so `export module` alone leaves it unreachable); got {math_mod:?}",
    );
    let root_mod = std::fs::read_to_string(src.join("mod.nu")).unwrap();
    assert!(
        root_mod.contains("export module math"),
        "root mod.nu should wire math; got {root_mod:?}",
    );
    assert!(
        !root_mod.contains("export use math"),
        "`math` is a PURE module (no `main`): an `export use` would flatten its \
         helpers into the parent; got {root_mod:?}",
    );
}

#[tested]
fn scaffold_names_only_the_files_it_wrote() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/namedlib");
    let first = s.scaffold(&["sourcetrait/namedlib:util", "sourcetrait/namedlib:util:echo"]);
    assert!(!has_error(&first), "got {first}");
    assert_eq!(
        created(&first),
        vec![src.join("util").join("mod.nu"), src.join("util").join("echo").join("mod.nu")],
        "a new module's mod.nu and a call's, each once; got {first}",
    );
    let second = s.scaffold(&["sourcetrait/namedlib:util:other"]);
    assert!(!has_error(&second), "got {second}");
    assert_eq!(
        created(&second),
        vec![src.join("util").join("other").join("mod.nu")],
        "a module walked through is no creation; got {second}",
    );
}

/// A scaffold's answer, its created paths.
fn created(env: &NuVal) -> Vec<std::path::PathBuf> {
    env["result"]["created"]
        .as_list()
        .expect("a created list")
        .iter()
        .map(|path| std::path::PathBuf::from(path.as_str().expect("a path")))
        .collect()
}

#[tested]
fn scaffolded_call_commits_as_wired() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/wiredlib");
    let _ = s.scaffold(&["sourcetrait/wiredlib:math:double"]);
    std::fs::write(
        src.join("math").join("double").join("mod.nu"),
        valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    )
    .unwrap();
    let env = s.commit("sourcetrait/wiredlib");
    assert!(
        !has_error(&env),
        "the scaffolder's own wiring must pass the validator; got {env}"
    );
    let called = s.call("sourcetrait/wiredlib:math:double", "{x: 21}");
    assert_eq!(
        called["result"]["out"].as_i64(),
        Some(42),
        "a scaffolded call must be drivable; got {called}",
    );
}

#[tested]
fn new_leaf_guard_refuses_existing_function() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let _ = s.new_rig(t.temp_dir(), "sourcetrait/guarded");
    let _ = s.scaffold(&["sourcetrait/guarded:m:f"]);
    let dup = s.scaffold(&["sourcetrait/guarded:m:f"]);
    assert!(
        has_error(&dup),
        "scaffolding over an existing function should reject; got {dup}"
    );
}

#[test]
fn scaffold_into_unregistered_rig_errors() {
    let s = TESTING.harness();
    let env = s.scaffold(&["sourcetrait/nopath:m:f"]);
    assert!(
        has_error(&env),
        "scaffolding into an unregistered rig should reject; got {env}"
    );
    assert_eq!(
        error_kind(&env),
        Some("equip::not_registered"),
        "got {env}"
    );
}

#[tested]
fn commit_validates_and_upserts_source() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/clib");
    let _ = s.scaffold(&["sourcetrait/clib:math:double"]);
    std::fs::write(
        src.join("math").join("double").join("mod.nu"),
        valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    )
    .unwrap();
    let env = s.commit("sourcetrait/clib");
    assert!(!has_error(&env), "commit should succeed; got {env}");
    assert!(
        s.rig_dir("sourcetrait/clib")
            .join("math")
            .join("double")
            .join("mod.nu")
            .exists(),
        "canonical should have the committed function",
    );
    let env2 = s.commit("sourcetrait/clib");
    assert!(!has_error(&env2), "no-change commit should succeed; got {env2}");
    let changed = &env2["result"];
    assert!(
        changed["added"].as_list().expect("added").is_empty()
            && changed["modified"].as_list().expect("modified").is_empty()
            && changed["removed"].as_list().expect("removed").is_empty(),
        "no-change commit should report nothing changed; got {env2}",
    );
}

#[tested]
fn commit_rejects_unfleshed_skeleton() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let _ = s.new_rig(t.temp_dir(), "sourcetrait/sklib");
    let _ = s.scaffold(&["sourcetrait/sklib:m:raw"]);
    let env = s.commit("sourcetrait/sklib");
    assert!(
        has_error(&env),
        "committing an unfleshed skeleton should reject; got {env}"
    );
    assert!(
        error_kinds(&env).iter().all(|k| k == "rig::contract"),
        "the skeleton's open records are the contract's refusals; got {env}",
    );
}

#[tested]
fn commit_rejects_main_in_flat_file() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/flatmainlib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use ./impl.nu\n");
    write_source(
        &src,
        "m/impl.nu",
        &valid_function_source("x: int", "out: int", "{ out: $data.x }"),
    );
    let env = s.commit("sourcetrait/flatmainlib");
    assert!(
        error_kinds(&env).iter().any(|k| k == "rig::main_in_flat_file"),
        "expected rig::main_in_flat_file; got {:?}",
        error_kinds(&env),
    );
}

#[tested]
fn commit_rejects_call_wired_via_export_module() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/wirelib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export module double\n");
    write_source(
        &src,
        "m/double/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
    let env = s.commit("sourcetrait/wirelib");
    assert!(
        error_kinds(&env).iter().any(|k| k == "rig::call_wiring"),
        "expected rig::call_wiring; got {:?}",
        error_kinds(&env),
    );
}

#[tested]
fn commit_rejects_call_with_submodule() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/leaflib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use call\n");
    write_source(
        &src,
        "m/call/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: $data.x }"),
    );
    write_source(&src, "m/call/sub/mod.nu", "");
    let env = s.commit("sourcetrait/leaflib");
    assert!(
        error_kinds(&env).iter().any(|k| k == "rig::call_leaf"),
        "expected rig::call_leaf; got {:?}",
        error_kinds(&env),
    );
}

#[tested]
fn commit_rejects_orphan_module() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/orphanlib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use double\n");
    write_source(
        &src,
        "m/double/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: $data.x }"),
    );
    write_source(&src, "m/stray/mod.nu", "");
    let env = s.commit("sourcetrait/orphanlib");
    assert!(
        error_kinds(&env).iter().any(|k| k == "rig::orphan"),
        "expected rig::orphan; got {:?}",
        error_kinds(&env),
    );
}

#[tested]
fn commit_accepts_nu_cmd_extra_command() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/extralib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export module snake\nexport use snake\n");
    write_source(
        &src,
        "m/snake/mod.nu",
        "export def main [data: record<text: string>]: nothing -> record<out: string> {\n    { out: ($data.text | str snake-case) }\n}\n",
    );
    let env = s.commit("sourcetrait/extralib");
    assert!(
        !has_error(&env),
        "a call-target using nu-cmd-extra (`str snake-case`) must commit; got {env}"
    );
    let called = s.call("sourcetrait/extralib:m:snake", "{text: 'Hello World'}");
    assert_eq!(
        called["result"]["out"].as_str(),
        Some("hello_world"),
        "the committed nu-cmd-extra call must run; got {called}",
    );
}

#[tested]
fn commit_accepts_call_with_flat_helper() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/callhelperlib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(&src, "m/thing/helper.nu", "export def doubler [n: int] { $n * 2 }\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        "use ./helper.nu *\nexport def main [data: record<x: int>]: nothing -> record<out: int> { { out: (doubler $data.x) } }\n",
    );
    let env = s.commit("sourcetrait/callhelperlib");
    assert!(
        !has_error(&env),
        "a call may use a flat helper sibling; got {env}"
    );
    let called = s.call("sourcetrait/callhelperlib:m:thing", "{x: 5}");
    assert_eq!(
        called["result"]["out"].as_i64(),
        Some(10),
        "call should run with the flat helper; got {called}",
    );
}
