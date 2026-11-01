
use std::os::unix::fs::PermissionsExt;

use sourcetrait_grammar_mcp::guts::{
    NuVal, TestServer, equip_source, error_kind, error_kinds, has_error, has_kind,
    valid_function_source, write_source,
};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary: constructing a TestServer runs
/// the namespace substrate (keypair, equip repo git config), which must not race
/// itself across parallel tests.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

fn author_double_tree(src: &std::path::Path) {
    write_source(src, "mod.nu", "export module m\n");
    write_source(src, "m/mod.nu", "export use double\n");
    write_source(
        src,
        "m/double/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
}

/// A lib module `text` exporting `shout`, the string upper-cased.
fn author_text_lib(lib: &std::path::Path) {
    write_source(lib, "text/mod.nu", "export def shout [s: string]: nothing -> string { $s | str upcase }\n");
}

#[tested]
fn install_brings_shipped_source_into_mcp() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/shiplib");
    author_double_tree(&eq.join("rig"));
    let env = s.equip("install", eq.to_str().unwrap());
    assert!(!has_error(&env), "install should succeed; got {env}");
    assert!(
        !env["result"]["summary"]["added"].as_list().expect("added").is_empty(),
        "install summary should report added paths; got {env}",
    );
    assert!(
        env["result"]["summary"]["warnings"].as_list().expect("warnings").is_empty(),
        "a clean source raises no warnings; got {env}",
    );
    assert!(
        s.rig_dir("sourcetrait/shiplib").exists(),
        "canonical should exist",
    );
    let called = s.call("sourcetrait/shiplib:m:double", "{x: 6}");
    assert_eq!(
        called["result"]["out"].as_i64(),
        Some(12),
        "installed rig should be callable; got {called}",
    );
}

#[tested]
fn install_writes_nothing_on_validation_failure() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/badship");
    let src = eq.join("rig");
    write_source(&src, "mod.nu", "export use thing\n");
    write_source(
        &src,
        "thing/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: $data.x }"),
    );
    let env = s.equip("install", eq.to_str().unwrap());
    assert_eq!(
        error_kind(&env),
        Some("rig::root_function"),
        "install of an invalid source should fail with a validation diagnostic; got {env}"
    );
    assert!(
        !s.rig_dir("sourcetrait/badship").exists(),
        "a failed install must leave nothing registered",
    );
    let re = s.equip("new", eq.to_str().unwrap());
    assert!(!has_error(&re), "name should be free after a failed install; got {re}");
}

#[tested]
fn install_carries_lib_and_rig_together() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/pairlib");
    author_text_lib(&eq.join("lib"));
    let rig = eq.join("rig");
    write_source(&rig, "mod.nu", "export module m\n");
    write_source(&rig, "m/mod.nu", "export use loud\n");
    write_source(
        &rig,
        "m/loud/mod.nu",
        "use equip/sourcetrait/pairlib/lib/text\nexport def main [data: record<s: string>]: nothing -> record<out: string> {\n    { out: (text shout $data.s) }\n}\n",
    );
    let env = s.equip("install", eq.to_str().unwrap());
    assert!(!has_error(&env), "a rig importing its own lib should install; got {env}");
    assert!(
        s.lib_dir("sourcetrait/pairlib").join("text/mod.nu").exists(),
        "the lib is committed beside the rig",
    );
    assert!(
        s.lib_dir("sourcetrait/pairlib").join(".meta/lib.nuon").exists(),
        "the lib carries its own index",
    );
    let called = s.call("sourcetrait/pairlib:m:loud", "{s: hi}");
    assert_eq!(
        called["result"]["out"].as_str(),
        Some("HI"),
        "the rig's call runs over its committed lib; got {called}",
    );
}

#[tested]
fn lib_only_equip_installs_and_imports_without_calls() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/onlylib");
    author_text_lib(&eq.join("lib"));
    let env = s.equip("install", eq.to_str().unwrap());
    assert!(!has_error(&env), "a lib alone is an equip; got {env}");

    let used = s.execute(
        "def execute [data: nothing]: nothing -> record<out: string> {\n    use equip/sourcetrait/onlylib/lib/text\n    { out: (text shout 'ok') }\n}",
        "null",
    );
    assert_eq!(
        used["result"]["out"].as_str(),
        Some("OK"),
        "a definition imports an installed lib by its namespace path; got {used}",
    );

    let called = s.call("sourcetrait/onlylib:text:shout", "{s: x}");
    assert_eq!(
        error_kind(&called),
        Some("equip::no_rig"),
        "an equip without a rig has no calls; got {called}",
    );
    let signatures = s.info()["result"]["signatures"].as_str().expect("signatures").to_string();
    assert!(
        !signatures.contains("onlylib"),
        "a lib-only equip lists no calls; got:\n{signatures}",
    );
}

#[tested]
fn a_diamond_shaped_lib_is_no_cycle() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/diamondlib");
    let lib = eq.join("lib");
    write_source(&lib, "base/mod.nu", "export def one []: nothing -> int { 1 }\n");
    write_source(&lib, "left/mod.nu", "use ../base\nexport def two []: nothing -> int { (base one) + 1 }\n");
    write_source(&lib, "right/mod.nu", "use ../base\nexport def three []: nothing -> int { (base one) + 2 }\n");
    write_source(
        &lib,
        "top/mod.nu",
        "use ../left\nuse ../right\nexport def sum []: nothing -> int { (left two) + (right three) }\n",
    );
    let rig = eq.join("rig");
    write_source(&rig, "mod.nu", "export module m\n");
    write_source(&rig, "m/mod.nu", "export module a\nexport use a\nexport module b\nexport use b\nexport module c\nexport use c\n");
    for call in ["a", "b", "c"] {
        write_source(
            &rig,
            &format!("m/{call}/mod.nu"),
            "use equip/sourcetrait/diamondlib/lib/top\nexport def main [data: nothing]: nothing -> record<out: int> {\n    { out: (top sum) }\n}\n",
        );
    }
    let env = s.equip("install", eq.to_str().unwrap());
    assert!(
        !has_error(&env),
        "a module shared down two branches registers more than once in a clean parse, \
         and that is no cycle; got {env}",
    );
    let called = s.call("sourcetrait/diamondlib:m:b", "null");
    assert_eq!(called["result"]["out"].as_i64(), Some(5), "got {called}");
}

#[tested]
fn install_skips_the_cli() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/clilib");
    author_double_tree(&eq.join("rig"));
    write_source(&eq.join("cli"), "mod.nu", "export def main [] { 'human' }\n");
    let env = s.equip("install", eq.to_str().unwrap());
    assert!(!has_error(&env), "install should succeed; got {env}");
    assert!(
        !s.rig_dir("sourcetrait/clilib").parent().unwrap().join("cli").exists(),
        "the MCP never takes an equip's cli",
    );
}

#[tested]
fn install_refuses_a_source_with_no_kinds() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/emptylib");
    std::fs::create_dir_all(eq.join("cli")).unwrap();
    let env = s.equip("install", eq.to_str().unwrap());
    assert_eq!(
        error_kind(&env),
        Some("equip::no_kinds"),
        "a source with neither lib/ nor rig/ installs nothing; got {env}",
    );
}

#[test]
fn equip_refuses_a_source_outside_an_equip_dir() {
    let s = TESTING.harness();
    for bad in ["/some/path", "/x/equip/sourcetrait", "/x/equip/sourcetrait/../y"] {
        let env = s.equip("install", bad);
        assert_eq!(
            error_kind(&env),
            Some("equip::invalid_source"),
            "`{bad}` names no equip; got {env}",
        );
    }
}

#[tested]
fn lib_denies_a_non_nu_file() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/libextlib");
    author_text_lib(&eq.join("lib"));
    write_source(&eq.join("lib"), "README.md", "# no\n");
    let env = s.equip("install", eq.to_str().unwrap());
    assert!(
        has_kind(&env, "lib::extension_denied"),
        "a lib holds .nu files alone; got {:?}",
        error_kinds(&env),
    );
}

#[tested]
fn lib_denies_an_executable_file() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/libxlib");
    author_text_lib(&eq.join("lib"));
    let file = eq.join("lib/text/mod.nu");
    let mut perms = std::fs::metadata(&file).unwrap().permissions();
    perms.set_mode(perms.mode() | 0o755);
    std::fs::set_permissions(&file, perms).unwrap();
    let env = s.equip("install", eq.to_str().unwrap());
    assert!(
        has_kind(&env, "lib::executable_denied"),
        "a +x lib file should be denied; got {:?}",
        error_kinds(&env),
    );
}

#[tested]
fn lib_reports_a_parse_error() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/libparselib");
    write_source(&eq.join("lib"), "broken/mod.nu", "export def f [] {\n");
    let env = s.equip("install", eq.to_str().unwrap());
    assert!(
        has_kind(&env, "lib::parse_error"),
        "a lib that does not parse is refused; got {:?}",
        error_kinds(&env),
    );
}

#[tested]
fn commit_mirrors_a_kind_removed_from_the_source() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/mirrorlib");
    author_text_lib(&eq.join("lib"));
    author_double_tree(&eq.join("rig"));
    let env = s.equip("install", eq.to_str().unwrap());
    assert!(!has_error(&env), "install should succeed; got {env}");
    std::fs::remove_dir_all(eq.join("rig")).unwrap();
    let committed = s.commit("sourcetrait/mirrorlib");
    assert!(!has_error(&committed), "commit should succeed; got {committed}");
    assert!(
        !s.rig_dir("sourcetrait/mirrorlib").exists(),
        "a kind gone from the source leaves the namespace",
    );
    assert!(
        s.lib_dir("sourcetrait/mirrorlib").join("text/mod.nu").exists(),
        "the kind still in the source stays",
    );
}

#[tested]
fn check_reports_ok_for_clean_source() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/checkoklib");
    author_double_tree(&src);
    let env = s.equip("check", src.parent().unwrap().to_str().unwrap());
    assert!(!has_error(&env), "check should not error; got {env}");
    let summary = &env["result"]["summary"];
    assert_eq!(summary["ok"].as_bool(), Some(true), "got {summary}");
    assert!(
        summary["errors"].as_list().expect("errors").is_empty(),
        "got {summary}"
    );
    assert!(
        summary["warnings"].as_list().expect("warnings").is_empty(),
        "got {summary}"
    );
}

#[tested]
fn check_reports_structural_errors() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/checkerrlib");
    write_source(&src, "mod.nu", "export use thing\n");
    write_source(
        &src,
        "thing/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: $data.x }"),
    );
    let env = s.equip("check", src.parent().unwrap().to_str().unwrap());
    assert!(!has_error(&env), "check itself should not error; got {env}");
    let summary = &env["result"]["summary"];
    assert_eq!(summary["ok"].as_bool(), Some(false), "got {summary}");
    let err_kinds: Vec<&str> = summary["errors"]
        .as_list()
        .expect("errors array")
        .iter()
        .filter_map(|e| e["kind"].as_str())
        .collect();
    assert!(!err_kinds.is_empty(), "expected >=1 error; got {summary}");
    assert!(
        err_kinds.iter().any(|k| k.starts_with("rig::")),
        "errors should carry namespaced rig:: kinds; got {err_kinds:?}"
    );
}

#[tested]
fn check_unregistered_equip_errors() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/ghostlib");
    let env = s.equip("check", eq.to_str().unwrap());
    assert_eq!(
        error_kind(&env),
        Some("equip::not_registered"),
        "check requires a registered equip; got {env}"
    );
}

#[tested]
fn check_source_dir_mismatch_errors() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let _ = s.new_rig(t.temp_dir(), "sourcetrait/checkmmlib");
    let elsewhere = equip_source(&t.temp_dir().join("elsewhere"), "sourcetrait/checkmmlib");
    let env = s.equip("check", elsewhere.to_str().unwrap());
    assert_eq!(
        error_kind(&env),
        Some("equip::source_path_mismatch"),
        "check should cross-check source_dir; got {env}"
    );
}

#[tested]
fn uninstall_source_dir_mismatch_errors() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let _ = s.new_rig(t.temp_dir(), "sourcetrait/unmmlib");
    let elsewhere = equip_source(&t.temp_dir().join("elsewhere"), "sourcetrait/unmmlib");
    let env = s.equip("uninstall", elsewhere.to_str().unwrap());
    assert_eq!(
        error_kind(&env),
        Some("equip::source_path_mismatch"),
        "uninstall should cross-check source_dir; got {env}"
    );
    assert!(
        s.rig_dir("sourcetrait/unmmlib").exists(),
        "a rejected uninstall must leave the equip registered",
    );
}

#[test]
fn invalid_action_errors() {
    let s = TESTING.harness();
    let env = s.equip("frobnicate", "/p/equip/sourcetrait/x");
    assert_eq!(
        error_kind(&env),
        Some("interface::invalid"),
        "an unknown action is no interface command; got {env}"
    );
}

#[tested]
fn a_definition_can_use_a_committed_rig() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/uselib");
    write_source(&src, "mod.nu", "export module calc\n");
    write_source(&src, "calc/mod.nu", "export use double\n");
    write_source(
        &src,
        "calc/double/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
    let committed = s.commit("sourcetrait/uselib");
    assert!(!has_error(&committed), "commit should succeed; got {committed}");

    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<out: int> {\n    use equip/sourcetrait/uselib/rig/calc\n    let r = (calc double {x: 5})\n    { out: $r.out }\n}",
        "null",
    );
    assert!(
        !has_error(&env),
        "a nu definition should be able to `use` a committed rig; got {env}"
    );
    assert_eq!(
        env["result"]["out"].as_i64(),
        Some(10),
        "the use should resolve + call the committed function; got {env}",
    );
}

#[tested]
fn install_and_commit_carry_the_validators_warnings() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/warnlib");
    let rig = eq.join("rig");
    author_double_tree(&rig);
    let long = "x".repeat(81);
    write_source(&rig, "mod.nu", &format!("# {long}\nexport module m\n"));
    let installed = s.equip("install", eq.to_str().unwrap());
    assert!(!has_error(&installed), "a warning never refuses an install; got {installed}");
    let warned = |env: &NuVal, at: &str| {
        let kinds: Vec<&str> = at
            .split('/')
            .skip(1)
            .try_fold(env, |value, key| value.get(key))
            .and_then(|warnings| warnings.as_list())
            .unwrap_or_else(|| panic!("no warnings at {at}; got {env}"))
            .iter()
            .filter_map(|warning| warning["kind"].as_str())
            .collect();
        assert!(
            kinds.contains(&"lint::summary_length"),
            "the over-long summary rides in the result; got {env}",
        );
    };
    warned(&installed, "/result/summary/warnings");

    let committed = s.commit("sourcetrait/warnlib");
    assert!(!has_error(&committed), "got {committed}");
    warned(&committed, "/result/warnings");
}
