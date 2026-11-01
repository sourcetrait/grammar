
use std::path::Path;

use sourcetrait_grammar_mcp::guts::{
    TestServer, has_error, rig_block, valid_function_source, write_source,
};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary: constructing a TestServer runs
/// the namespace substrate (keypair, equip repo git config), which must not race
/// itself across parallel tests.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

/// Read a committed golden under `<crate>/testing/goldens/`, comparing it to
/// `actual`. `BLESS=1 cargo test` (re)writes it from the actual value; without
/// the golden present (and no BLESS) the read fails loudly.
///
/// The signature block is WHITESPACE-SIGNIFICANT, which is what makes it a good
/// golden: an indentation bug shows up as a plain file diff rather than as a
/// structural assertion nobody reads.
fn golden_text(
    name: &str,
    actual: &str,
) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testing/goldens")
        .join(name);
    if std::env::var("BLESS").is_ok() {
        std::fs::create_dir_all(path.parent().unwrap()).expect("mkdir goldens");
        std::fs::write(&path, actual).expect("write golden");
    }
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read golden {name}: {e} (run with BLESS=1 to (re)generate)"))
}

fn signatures(s: &TestServer) -> String {
    s.info()["result"]["signatures"]
        .as_str()
        .expect("info carries a signatures block")
        .to_string()
}

#[test]
fn info_returns_static_server_state() {
    let s = TESTING.harness();
    let envelope = s.info();
    let env = &envelope["result"];
    assert_eq!(env["name"].as_str(), Some("grammar"), "name should be grammar; got {envelope}");

    let version = env["version"].as_str().expect("version field");
    assert_eq!(
        version,
        env!("CARGO_PKG_VERSION"),
        "version should match crate CARGO_PKG_VERSION",
    );

    let nu_version = env["nu_version"].as_str().expect("nu_version field");
    assert!(
        !nu_version.is_empty() && nu_version.contains('.'),
        "nu_version should be non-empty + semver-shaped; got {nu_version:?}",
    );

    let plugins = env["plugins"].as_list().expect("plugins should be an array");
    for p in plugins {
        let entry = p
            .as_list()
            .expect("each plugin is a positional [name, version] pair");
        assert_eq!(entry.len(), 2, "plugin entry is a 2-element [name, version]; got {entry:?}");
        let name = entry[0].as_str().expect("plugin name is a string");
        assert!(!name.is_empty(), "plugin name should be non-empty");
        assert!(
            entry[1].as_str().is_some() || entry[1].is_nothing(),
            "plugin version slot should be string-or-null; got {:?}",
            entry[1],
        );
    }

    assert!(
        env.get("rigs").is_none(),
        "the structured hierarchy is gone; `signatures` replaces it. got {env}",
    );
    assert!(env["signatures"].as_str().is_some(), "signatures is a text block; got {env}");
}

#[tested]
fn info_renders_a_committed_rig_as_a_signature_block() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/treelib");
    for (module_path, name, data_inner, result_inner, body) in [
        ("solo", "rootfn", "x: int", "out: int", "{ out: ($data.x + 1) }"),
        (
            "alpha",
            "a1",
            "s: string",
            "len: int",
            "{ len: ($data.s | str length) }",
        ),
        (
            "alpha/beta",
            "b1",
            "x: int, t: record<y: string, n: list<int>>",
            "out: record<y: string>",
            "{ out: { y: $data.t.y } }",
        ),
    ] {
        let np = format!("sourcetrait/treelib:{module_path}:{name}");
        let scaffold = s.scaffold(&[np.as_str()]);
        assert!(!has_error(&scaffold), "scaffold {np} failed: {scaffold}");
        let rel = format!("{module_path}/{name}/mod.nu");
        write_source(&src, &rel, &valid_function_source(data_inner, result_inner, body));
    }
    let committed = s.commit("sourcetrait/treelib");
    assert!(!has_error(&committed), "commit failed: {committed}");

    let block = signatures(s);
    let actual = rig_block(&block, "treelib");
    assert_eq!(
        actual,
        golden_text("info_treelib.txt", &actual),
        "the treelib signature block drifted from the golden",
    );
    assert!(
        block.contains("sourcetrait\n"),
        "the author heads its own group and carries no summary; got:\n{block}",
    );
}

#[tested]
fn the_indentation_is_the_hierarchy() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/implib");
    write_source(&src, "mod.nu", "export module math\n");
    write_source(&src, "math/mod.nu", "export use double\n");
    write_source(
        &src,
        "math/double/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
    let committed = s.commit("sourcetrait/implib");
    assert!(!has_error(&committed), "commit failed: {committed}");

    assert_eq!(
        rig_block(&signatures(s), "implib"),
        " implib\n  math\n   double <x:int> <out:int>\n",
        "one space per level, and an undocumented line carries no ` # `",
    );
}

#[tested]
fn summaries_ride_the_line_and_are_omitted_when_absent() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/doctreelib");
    write_source(&src, "mod.nu", "# the doctree rig\nexport module m\n");
    write_source(&src, "m/mod.nu", "# the m module\nexport use fn\n");
    write_source(
        &src,
        "m/fn/mod.nu",
        "# the fn summary\nexport def main [data: record<x: int>]: nothing -> record<out: int> { { out: $data.x } }\n",
    );
    let committed = s.commit("sourcetrait/doctreelib");
    assert!(!has_error(&committed), "commit failed: {committed}");

    assert_eq!(
        rig_block(&signatures(s), "doctreelib"),
        " doctreelib # the doctree rig\n  m # the m module\n   fn <x:int> <out:int> # the fn summary\n",
        "summary is part of the line, on every kind that has one",
    );
}

#[tested]
fn nothing_renders_as_its_type() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/nothinglib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use ping\n");
    write_source(
        &src,
        "m/ping/mod.nu",
        "export def main [data: nothing]: nothing -> nothing { }\n",
    );
    let committed = s.commit("sourcetrait/nothinglib");
    assert!(!has_error(&committed), "commit failed: {committed}");

    assert_eq!(
        rig_block(&signatures(s), "nothinglib"),
        " nothinglib\n  m\n   ping <nothing> <nothing>\n",
        "`nothing` is a type like any other, its group spelled whole",
    );
}

#[tested]
fn a_multi_line_summary_is_flattened_onto_its_line() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/wraplib");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use go\n");
    // A doc comment whose SUMMARY - everything before the first blank line -
    // spans two source lines. The parser joins those with a newline, and real
    // rigs in the namespace are written this way.
    write_source(
        &src,
        "m/go/mod.nu",
        "# Runs the thing against the other thing,\n# gating each step as it goes.\nexport def main [data: record<x: int>]: nothing -> record<out: int> { { out: $data.x } }\n",
    );
    let committed = s.commit("sourcetrait/wraplib");
    assert!(!has_error(&committed), "commit failed: {committed}");

    let block = rig_block(&signatures(s), "wraplib");
    assert_eq!(
        block,
        " wraplib\n  m\n   go <x:int> <out:int> # Runs the thing against the other thing, gating each step as it goes.\n",
        "a two-line summary must be FLATTENED onto its own line",
    );
    assert_eq!(
        block.lines().count(),
        3,
        "three signatures means three lines - an unflattened summary would add a \
         fourth at column 0, which the grammar reads as an AUTHOR; got:\n{block}",
    );
}

#[tested]
fn a_mixed_module_states_its_own_call_separator() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/mixedlib");
    write_source(&src, "mod.nu", "export module m\n");
    // `m` holds BOTH a submodule and a direct call. This is the case that sank
    // the trailing-character format - no single character describes it - and it
    // needs NOTHING special here, because a call is recognizable by its own
    // shape rather than by what its parent announced.
    write_source(&src, "m/mod.nu", "export module deep\nexport use here\n");
    write_source(
        &src,
        "m/here/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: $data.x }"),
    );
    write_source(&src, "m/deep/mod.nu", "export use down\n");
    write_source(
        &src,
        "m/deep/down/mod.nu",
        &valid_function_source("y: int", "out: int", "{ out: $data.y }"),
    );
    let committed = s.commit("sourcetrait/mixedlib");
    assert!(!has_error(&committed), "commit failed: {committed}");

    assert_eq!(
        rig_block(&signatures(s), "mixedlib"),
        " mixedlib\n  m\n   here <x:int> <out:int>\n   deep\n    down <y:int> <out:int>\n",
        "a mixed module needs no marker: `here` is a call because it CARRIES the \
         two signature groups, so a reader knows the module path ended at `m` \
         and reads sourcetrait/mixedlib:m:here, while `deep` continues it",
    );
}

#[tested]
fn an_author_heads_its_group_exactly_once() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    for leaf in ["onelib", "twolib"] {
        let name = format!("sourcetrait/{leaf}");
        let src = s.new_rig(t.temp_dir(), &name);
        write_source(&src, "mod.nu", "export module m\n");
        write_source(&src, "m/mod.nu", "export use go\n");
        write_source(
            &src,
            "m/go/mod.nu",
            &valid_function_source("x: int", "out: int", "{ out: $data.x }"),
        );
        let committed = s.commit(&name);
        assert!(!has_error(&committed), "commit {name} failed: {committed}");
    }
    let block = signatures(s);
    let heads = block.lines().filter(|l| *l == "sourcetrait").count();
    assert_eq!(
        heads, 1,
        "the author line is emitted once per GROUP, not once per rig; got:\n{block}",
    );
}
