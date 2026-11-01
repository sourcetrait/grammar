
use sourcetrait_grammar_mcp::guts::{
    TestServer, error_kind, error_messages, has_error, valid_function_source, write_source,
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
fn a_builtin_is_reached_by_its_bare_name() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/gram");
    let scaffolded = s.scaffold(&["gram:m:tally"]);
    assert!(!has_error(&scaffolded), "new() takes a builtin's bare name; got {scaffolded}");
    write_source(&src, "mod.nu", "# the gram rig\nexport module m\n");
    write_source(
        &src,
        "m/tally/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x + 1) }"),
    );
    let committed = s.commit("gram");
    assert!(!has_error(&committed), "commit() takes a builtin's bare name; got {committed}");

    let called = s.call("gram:m:tally", "{x: 41}");
    assert_eq!(called["result"]["out"].as_i64(), Some(42), "got {called}");
    let full = s.call("sourcetrait/gram:m:tally", "{x: 1}");
    assert_eq!(full["result"]["out"].as_i64(), Some(2), "the full name still calls; got {full}");

    let inspected = s.inspect("gram:m:tally");
    assert_eq!(
        inspected["result"]["doc"]["signature"].as_str(),
        Some("gram:m:tally <x:int> <out:int>"),
        "a standalone signature names a builtin bare; got {inspected}",
    );

    let block = "gram # the gram rig\n m\n  tally <x:int> <out:int>\n";
    let info = s.info();
    let signatures = info["result"]["signatures"].as_str().expect("signatures block");
    assert!(
        signatures.starts_with(block),
        "a builtin leads the block at an author's depth by its bare name; got:\n{signatures}",
    );
    assert!(
        !signatures.lines().any(|line| line == "sourcetrait"),
        "a builtin is never listed under its author; got:\n{signatures}",
    );

    let pattern = s.inspect("gram:");
    assert_eq!(pattern["result"]["doc"]["signatures"].as_str(), Some(block), "got {pattern}");

    let missing = s.call("gram:m:nosuch", "null");
    assert_eq!(error_kind(&missing), Some("function::not_defined"), "got {missing}");
    assert_eq!(
        error_messages(&missing),
        vec!["function `nosuch` is not defined in `gram:m`".to_string()],
        "a refusal names a builtin bare, as every other surface does; got {missing}",
    );

    let configured = s.purview_configure("gram_calls", &["gram:m:"]);
    assert!(!has_error(&configured), "a purview takes a builtin's bare form; got {configured}");
    assert_eq!(configured["result"]["signatures"].as_str(), Some(block), "got {configured}");
}

#[tested]
fn an_uninstalled_builtin_takes_its_bare_patterns_with_it() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/grammar");
    let _ = s.scaffold(&["grammar:eye:dir"]);
    write_source(
        &src,
        "eye/dir/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: $data.x }"),
    );
    let committed = s.commit("grammar");
    assert!(!has_error(&committed), "got {committed}");
    let configured = s.purview_configure("grammar_view", &["grammar:", "grammar:eye:dir"]);
    assert!(!has_error(&configured), "got {configured}");

    let source = src.parent().expect("a rig source sits in its equip source");
    let removed = s.equip("uninstall", &source.to_string_lossy());
    assert!(!has_error(&removed), "got {removed}");
    let purviews = s.purviews();
    let ids: Vec<&str> = purviews["result"]["purviews"]
        .as_list()
        .expect("purviews list")
        .iter()
        .filter_map(|row| row[0].as_str())
        .collect();
    assert!(
        !ids.contains(&"grammar_view"),
        "both bare patterns dangle once the builtin goes, so the row goes; got {purviews}",
    );
}
