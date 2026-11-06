//! The `bed` tool end to end through the in-process harness: an embedded command
//! run as a call of its own, held to the interface's rules.

use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_mcp::guts::{NuVal, TestServer, error_kind, has_error, nuon};

static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

/// A successful envelope's result, panicking on an error envelope.
fn result_of(env: &NuVal) -> &NuVal {
    assert!(!has_error(env), "got {env}");
    &env["result"]
}

#[test]
fn a_grmr_command_runs_with_or_without_its_word() {
    let s = TESTING.harness();
    for subcommand in ["config list", "grmr config list", "  config list"] {
        let config = s.bed("grmr", subcommand);
        assert!(result_of(&config).as_list().is_some(), "`{subcommand}`: {config}");
        assert!(config["nonce"].as_str().is_some(), "the bed call mints its nonce; got {config}");
    }
}

#[test]
fn a_bed_call_is_a_call_of_its_own() {
    let s = TESTING.harness();
    let traced = s.bed("grmr", "dbg {a: 1, b: 'two'}");
    assert!(result_of(&traced).is_nothing(), "got {traced}");
    let nonce = traced["nonce"].as_str().expect("its nonce");
    let log = s.calls_log_dir(nonce).join("debug.nuonl");
    let debug = std::fs::read_to_string(&log).unwrap_or_else(|e| panic!("{}: {e}", log.display()));
    assert_eq!(debug.lines().collect::<Vec<_>>(), vec!["{a: 1, b: two}"], "dbg writes its own call's log");
}

#[test]
fn a_grmr_command_that_fails_answers_its_eval_error() {
    let s = TESTING.harness();
    let refused = s.bed("grmr", "config pin 'supervisor.nosuch' 1 0.5");
    assert_eq!(error_kind(&refused), Some("thread::returned_error"), "it ran and failed; got {refused}");
    assert!(refused["nonce"].as_str().is_some(), "got {refused}");
}

#[test]
fn a_bed_call_is_refused_beyond_its_one_literal_command() {
    let s = TESTING.harness();
    for (command, subcommand) in [
        ("grmr", "dbg {a: 1} | to nuon"),
        ("grmr", "dbg {a: 1}; grmr dbg {b: 2}"),
        ("grmr", "dbg $env"),
        ("grmr", "dbg (ls)"),
        ("grmr", "dbg {a: (1 + 1)}"),
        ("grmr", "ls"),
        ("grmr", ""),
        ("nu", "ls"),
        ("interface", "info"),
    ] {
        let env = s.bed(command, subcommand);
        assert_eq!(error_kind(&env), Some("bed::invalid"), "`{command}` `{subcommand}`: {env}");
        assert!(env["nonce"].as_str().is_some(), "`{command}` `{subcommand}`: {env}");
    }
    for subcommand in ["dbg $env", "grmr dbg $env"] {
        let refused = s.bed("grmr", subcommand);
        let col = subcommand.find("$env").expect("the variable") + 1;
        assert_eq!(
            refused["error"][0]["source"]["position"],
            nuon(&format!("[1, {col}]")),
            "positioned in the subcommand as sent; got {refused}",
        );
    }
}
