//! A bed call's command held to the rules: one embedded call, literal arguments.
//!
//! The parse engine is the lint's, started on homes of the binary's own, since
//! `init` holds once.

use std::path::Path;

use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_engine::{
    EngineIdentity, EngineParams, GRMR_COMMANDS, KIND_BED, ParseEngine, analyze_bed,
    grmr_signatures, init,
};

static TESTING: testing::ModuleWith<ParseEngine> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|module| parse_engine(module.temp_dir()))
});

/// The engine's params set on homes under `root`, then a parse engine as the lint's.
fn parse_engine(root: &Path) -> ParseEngine {
    let params = EngineParams {
        id: "test".to_string(),
        namespace: "bed".to_string(),
        work_dir: root.join("work"),
        tmp_dir: root.join("tmp"),
        transient_dir: root.join("transient"),
        data_home: root.join("data"),
        cache_home: root.join("cache"),
        identity: EngineIdentity::draw(),
    };
    for dir in [
        &params.work_dir,
        &params.tmp_dir,
        &params.transient_dir,
        &params.data_home,
        &params.cache_home,
    ] {
        std::fs::create_dir_all(dir).expect("a home for the engine");
    }
    init(params).expect("the engine's init");
    ParseEngine::new_full()
}

#[test]
fn the_grmr_commands_are_the_signature_blocks_names() {
    let block = grmr_signatures();
    let names: Vec<&str> = block
        .lines()
        .filter_map(|line| line.strip_prefix('`'))
        .filter_map(|line| line.split(" [").next())
        .collect();
    assert_eq!(names, GRMR_COMMANDS);
}

#[test]
fn a_grmr_command_holds_with_or_without_its_word_and_literal_arguments() {
    let engine = TESTING.harness();
    for (subcommand, text) in [
        ("dbg {a: 1, b: [x, 2]}", "grmr dbg {a: 1, b: [x, 2]}"),
        ("grmr dbg [[x, y]; [1, 2]]", "grmr dbg [[x, y]; [1, 2]]"),
        (
            "channel send 'my/Thing' {n: 1.5, on: true, none: null, at: 2026-11-02, took: 3sec}",
            "grmr channel send 'my/Thing' {n: 1.5, on: true, none: null, at: 2026-11-02, took: 3sec}",
        ),
        ("get_config 'supervisor.cpu'", "grmr get_config 'supervisor.cpu'"),
        ("pin_config 'supervisor.cpu' 1234 0.5", "grmr pin_config 'supervisor.cpu' 1234 0.5"),
        ("  get_config_all", "grmr   get_config_all"),
    ] {
        let held = analyze_bed(engine, "grmr", subcommand).unwrap_or_else(|rows| panic!("`{subcommand}`: {rows:?}"));
        assert_eq!(held, text, "{subcommand}");
    }
}

#[test]
fn a_bed_call_is_refused_beyond_its_one_literal_command() {
    let engine = TESTING.harness();
    for subcommand in [
        "dbg {a: 1} | to nuon",
        "dbg {a: 1}; grmr dbg {b: 2}",
        "dbg $env",
        "dbg (ls)",
        "dbg {a: (1 + 1)}",
        "dbg [(ls)]",
        "ls",
        "",
        "dbg {a: 1} o> out.nuon",
    ] {
        let rows = analyze_bed(engine, "grmr", subcommand).expect_err(subcommand);
        assert!(!rows.is_empty() && rows.iter().all(|row| row.kind == KIND_BED), "`{subcommand}`: {rows:?}");
    }
    let rows = analyze_bed(engine, "nu", "ls").expect_err("no embedded command");
    assert_eq!((rows[0].kind.as_str(), rows[0].source.is_none()), (KIND_BED, true), "{rows:?}");
    assert!(rows[0].message.contains("`grmr`"), "the refusal names the embedded commands: {rows:?}");
}

#[test]
fn a_refusal_is_positioned_in_the_subcommand_as_sent() {
    let engine = TESTING.harness();
    for subcommand in ["dbg $env", "grmr dbg $env", "  dbg $env"] {
        let rows = analyze_bed(engine, "grmr", subcommand).expect_err(subcommand);
        let col = subcommand.find("$env").expect("the variable") as u64 + 1;
        assert_eq!(rows[0].source.as_ref().map(|s| s.position), Some([1, col]), "{subcommand}: {rows:?}");
    }
}
