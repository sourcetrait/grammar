//! Lifecycle SYSTEM tests for the conditions a nu definition cannot trigger on its
//! own (a caught panic; a hung engine thread). They spawn the real host built with
//! the mcp `test-hooks` feature (__test_panic / __test_hang) and are themselves
//! gated behind this crate's own, so plain `cargo test --workspace` (which builds
//! the host without the hooks) skips them. Run:
//!   cargo build -p sourcetrait_grammar_mcp --features test-hooks
//!   cargo test -p sourcetrait_grammar_tests --features test-hooks -- --test-threads=1
#![cfg(feature = "test-hooks")]

use std::time::{Duration, Instant};

use serde_json::json;
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// Poll the per-process emergency log(s) under the namespace's `log/` dir until one
/// carries a hung-thread record, or `secs` elapse; returns the matching (or the
/// last-seen) content. The `<mcp_nom>` subdir is minted at startup, so glob it.
fn wait_for_hung_emergency(host: &Host, id: &str, ns: &str, secs: u64) -> String {
    let log_root = namespace_dir(host.cache_home(), id, ns).join("log");
    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut last = String::new();
    loop {
        if let Ok(entries) = std::fs::read_dir(&log_root) {
            for e in entries.flatten() {
                if let Ok(content) = std::fs::read_to_string(e.path().join("emergency.nuonl")) {
                    last = content;
                    if last.contains("hung_engine_thread") {
                        return last;
                    }
                }
            }
        }
        if Instant::now() >= deadline {
            return last;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// A definition on `lane` (`def` or `def --env`) answering `result_type` from `body`.
fn definition(lane: &str, result_type: &str, body: &str) -> String {
    format!("{lane} execute [data: nothing]: nothing -> {result_type} {{\n{body}\n}}")
}

#[tested]
fn a_panic_in_the_pool_is_caught_and_the_host_survives() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());
    // __test_panic panics the eval thread; catch_unwind catches it -> the call
    // errors, the host is NOT torn down.
    let env = host.execute(&definition("def", "record<out: int>", "__test_panic\n{ out: 0 }"), "null");
    assert!(
        has_error_path(&env),
        "a panicking eval should surface as an error, not crash the host; got {env}",
    );
    // The host is alive: a normal call still works.
    let ok = host.execute(
        "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x + 1) } }",
        "{x: 5}",
    );
    assert_eq!(
        envelope(&ok)["result"]["out"].as_i64(),
        Some(6),
        "the host should survive the caught panic; got {ok}",
    );
}

#[tested]
fn a_stateful_panic_resets_the_session_and_keeps_the_host() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());
    // Seed persistent session state.
    let a = host.execute(
        &definition("def --env", "record<ok: bool>", "$env.KEEP = 'before'\n{ ok: true }"),
        "null",
    );
    assert!(!has_error_path(&a), "the seed call should succeed; got {a}");
    // Panic the stateful eval -> caught, the lane rebuilds its engine (session
    // lost), the host survives.
    let p = host.execute(&definition("def --env", "record<ok: bool>", "__test_panic\n{ ok: true }"), "null");
    assert!(has_error_path(&p), "a panicking stateful eval should surface as an error; got {p}");
    // Host + lane survive, and the pre-panic session state is GONE (reset).
    let b = host.execute(
        &definition("def --env", "record<keep: string>", "{ keep: ($env.KEEP? | default 'gone') }"),
        "null",
    );
    assert!(!has_error_path(&b), "the stateful lane should work after the reset; got {b}");
    assert_eq!(
        envelope(&b)["result"]["keep"].as_str(),
        Some("gone"),
        "the reset should have dropped the pre-panic session state; got {b}",
    );
}

#[tested]
fn hung_thread_is_confirmed_in_emergency_log() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--id", "hid"]);
    // __test_hang blocks uninterruptibly; a short timeout triggers cancel, but the
    // bare sleep never polls Signals, so the thread outlives its cancel and the
    // watchdog confirms it a hung engine thread (past the 5s grace) and logs it.
    let mut args = execute_args(&definition("def", "record<out: int>", "__test_hang\n{ out: 0 }"), "null");
    args["timeout"] = json!("500ms");
    let timed = host.call("execute", args);
    assert_eq!(envelope_error_kind(&timed).as_deref(), Some("thread::timeout"), "got {timed}");
    let nonce = envelope(&timed)["nonce"]
        .as_str()
        .expect("a timeout's envelope carries its nonce")
        .to_string();

    // The watchdog samples every 2s and confirms past a 5s grace; wait it out.
    let log = wait_for_hung_emergency(&host, "hid", "default", 15);
    let hit = log
        .lines()
        .find(|l| l.contains("hung_engine_thread") && l.contains(&nonce));
    assert!(
        hit.is_some(),
        "expected a hung_engine_thread emergency for nonce {nonce}; log=\n{log}",
    );
    assert!(
        hit.unwrap().contains("stateless"),
        "the hang should be lane=stateless; got {}",
        hit.unwrap(),
    );
}
