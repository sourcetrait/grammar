//! The `grimm` config surface, driven from real `execute` definitions.
//!
//! In-process is the right tier for these, and not merely the cheap one: the pin
//! registry is a process-global, and `TestServer` shares the very process the
//! eval runs in, so a test can pin to its own pid and know for certain the
//! process is alive - which is the one thing a spawned host could not give it.

use sourcetrait_grammar_mcp::guts::{
    TestServer, clear_config_pins, error_text, has_error, nuon,
};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary (substrate init must not race
/// itself). Every case below mutates or reads the process-global pin registry
/// through eval bodies, so the whole file runs as one Stepper-sequenced test.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .setup(|_| TestServer::new())
});

#[tested]
fn grimm_config_surface_is_sequenced() {
    testing::Stepper::builder("grimm_config_surface")
        .init(|_t, _o: ()| {
            clear_config_pins();
            testing::StepState((), ())
        })
        .step("get_config_all_mirrors_the_toml_tables", |t, s, p| {
            get_config_all_mirrors_the_toml_tables(t);
            testing::StepState(s, p)
        })
        .step("a_dotted_key_is_a_path_into_the_record", |t, s, p| {
            a_dotted_key_is_a_path_into_the_record(t);
            testing::StepState(s, p)
        })
        .step("an_unknown_key_errors_and_names_the_valid_ones", |t, s, p| {
            an_unknown_key_errors_and_names_the_valid_ones(t);
            testing::StepState(s, p)
        })
        .step("an_unpinned_port_reads_as_null_rather_than_a_sentinel", |t, s, p| {
            an_unpinned_port_reads_as_null_rather_than_a_sentinel(t);
            testing::StepState(s, p)
        })
        .step("a_pin_changes_what_the_config_surface_reports", |t, s, p| {
            a_pin_changes_what_the_config_surface_reports(t);
            testing::StepState(s, p)
        })
        .step("the_motivating_case_is_zero_vram_headroom", |t, s, p| {
            the_motivating_case_is_zero_vram_headroom(t);
            testing::StepState(s, p)
        })
        .step("last_write_wins_across_calls", |t, s, p| {
            last_write_wins_across_calls(t);
            testing::StepState(s, p)
        })
        .step("only_the_supervisor_lines_are_pinnable", |t, s, p| {
            only_the_supervisor_lines_are_pinnable(t);
            testing::StepState(s, p)
        })
        .step("a_pin_is_held_to_the_config_files_own_bounds", |t, s, p| {
            a_pin_is_held_to_the_config_files_own_bounds(t);
            testing::StepState(s, p)
        })
        .step("a_pin_against_a_dead_process_is_refused", |t, s, p| {
            a_pin_against_a_dead_process_is_refused(t);
            testing::StepState(s, p)
        })
        .step("the_config_decls_reach_the_stateful_lane_too", |t, s, p| {
            the_config_decls_reach_the_stateful_lane_too(t);
            testing::StepState(s, p)
        })
        .finalize()
        .run(TESTING.as_testable(), ());
}

/// The pid the eval actually runs under. Eval is in-process, so the test's own
/// pid IS the host's.
fn me() -> u32 {
    std::process::id()
}

/// A stateless definition taking nothing, answering `result_type`.
fn reading(result_type: &str, body: &str) -> String {
    format!("def execute [data: nothing]: nothing -> {result_type} {{\n{body}\n}}")
}

/// A stateless definition taking this process's pid as `$data.pid`.
fn pinning(result_type: &str, body: &str) -> String {
    format!("def execute [data: record<pid: int>]: nothing -> {result_type} {{\n{body}\n}}")
}

/// The data `pinning` takes.
fn my_pid() -> String {
    format!("{{pid: {}}}", me())
}

fn get_config_all_mirrors_the_toml_tables(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    let env = s.execute(
        &reading(
            "record<tables: list<string>, channel_keys: list<string>, supervisor_keys: list<string>, \
             identity_absent: bool>",
            "let cfg = (grimm get_config_all)\n\
             { tables: ($cfg | columns), channel_keys: ($cfg.channel | columns), \
               supervisor_keys: ($cfg.supervisor | columns), \
               identity_absent: ($cfg.identity == null) }",
        ),
        "null",
    );
    assert!(!has_error(&env), "get_config_all should run; got {env}");
    assert_eq!(
        env["result"]["tables"],
        nuon("[channel, supervisor, identity]"),
        "the record mirrors the TOML's tables; got {env}",
    );
    assert_eq!(
        env["result"]["identity_absent"].as_bool(),
        Some(true),
        "a host whose file names no [identity] reads it as null; got {env}",
    );
    assert_eq!(
        env["result"]["channel_keys"],
        nuon(
            "[port, spam_warn_window_secs, spam_warn_rate, spam_error_window_secs, \
             spam_error_rate]",
        ),
        "got {env}",
    );
    assert_eq!(
        env["result"]["supervisor_keys"],
        nuon(
            "[cpu_warn_fraction, ram_warn_fraction, vram_warn_headroom_mib, \
             disk_warn_fraction]",
        ),
        "got {env}",
    );
}

fn a_dotted_key_is_a_path_into_the_record(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    // The coherence property: the key is a path into what get_config_all returns,
    // not a parallel naming scheme. If these ever disagree, one of the two
    // surfaces is lying about where a setting lives.
    //
    // The walked form uses a literal cell path, which is what an author writes.
    // A string variable would not do: `get $key` binds a one-member cell path
    // whose single member is the whole string, dots included, so it looks for a
    // column literally named "supervisor.cpu_warn_fraction" and fails. Splitting
    // a string on dots is the source parser's job, not `get`'s.
    let env = s.execute(
        &reading(
            "record<agree: bool, value: float>",
            "let direct = (grimm get_config 'supervisor.cpu_warn_fraction')\n\
             let walked = (grimm get_config_all | get supervisor.cpu_warn_fraction)\n\
             { agree: ($direct == $walked), value: $direct }",
        ),
        "null",
    );
    assert!(!has_error(&env), "got {env}");
    assert_eq!(env["result"]["agree"].as_bool(), Some(true), "got {env}");
    assert_eq!(
        env["result"]["value"].as_f64(),
        Some(0.80),
        "the embedded default; got {env}",
    );
}

fn an_unknown_key_errors_and_names_the_valid_ones(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    let env = s.execute(
        &reading("record<out: int>", "grimm get_config 'supervisor.nonesuch'\n{ out: 0 }"),
        "null",
    );
    // An error rather than a null: a typo and a genuinely-null setting are
    // indistinguishable at the call site, and only one is a bug worth hearing about.
    assert!(has_error(&env), "an unknown key must error; got {env}");
    let text = error_text(&env);
    assert!(text.contains("not a config key"), "got {text}");
    assert_eq!(
        text.matches("supervisor.vram_warn_headroom_mib").count(),
        1,
        "the error should carry the vocabulary, once; got {text}",
    );
}

fn an_unpinned_port_reads_as_null_rather_than_a_sentinel(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    let env = s.execute(
        &reading(
            "record<is_null: bool>",
            "{ is_null: ((grimm get_config 'channel.port') == null) }",
        ),
        "null",
    );
    assert_eq!(
        env["result"]["is_null"].as_bool(),
        Some(true),
        "an absent port is nothing, never a 0 that reads as a real port; got {env}",
    );
}

fn a_pin_changes_what_the_config_surface_reports(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    clear_config_pins();
    let env = s.execute(
        &pinning(
            "record<before: float, after: float>",
            "let key = 'supervisor.cpu_warn_fraction'\n\
             let before = (grimm get_config $key)\n\
             grimm pin_config $key $data.pid 0.25\n\
             { before: $before, after: (grimm get_config $key) }",
        ),
        &my_pid(),
    );
    assert!(!has_error(&env), "pin_config should run; got {env}");
    assert_eq!(env["result"]["before"].as_f64(), Some(0.80), "got {env}");
    assert_eq!(
        env["result"]["after"].as_f64(),
        Some(0.25),
        "a pin IS the setting - every reader goes through the same layer; got {env}",
    );
    clear_config_pins();
}

fn the_motivating_case_is_zero_vram_headroom(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    clear_config_pins();
    // Pin the headroom to 0 for a training run: the gate fires at
    // `total - headroom`, so zero puts the line at the card's full capacity and
    // usage never reaches it.
    let env = s.execute(
        &pinning(
            "record<headroom: int>",
            "grimm pin_config 'supervisor.vram_warn_headroom_mib' $data.pid 0\n\
             { headroom: (grimm get_config 'supervisor.vram_warn_headroom_mib') }",
        ),
        &my_pid(),
    );
    assert!(!has_error(&env), "zero headroom must be legal; got {env}");
    assert_eq!(env["result"]["headroom"].as_i64(), Some(0), "got {env}");
    clear_config_pins();
}

fn last_write_wins_across_calls(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    clear_config_pins();
    for value in ["0.5", "0.9"] {
        let env = s.execute(
            &pinning(
                "record<ok: bool>",
                &format!(
                    "grimm pin_config 'supervisor.disk_warn_fraction' $data.pid {value}\n\
                     {{ ok: true }}",
                ),
            ),
            &my_pid(),
        );
        assert!(!has_error(&env), "pin {value} should run; got {env}");
    }
    let env = s.execute(
        &reading(
            "record<value: float>",
            "{ value: (grimm get_config 'supervisor.disk_warn_fraction') }",
        ),
        "null",
    );
    assert_eq!(
        env["result"]["value"].as_f64(),
        Some(0.9),
        "a pin on a key replaces the one before it; got {env}",
    );
    clear_config_pins();
}

fn only_the_supervisor_lines_are_pinnable(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    clear_config_pins();
    // `interface channel config` stays the sole mutator of the channel settings; a
    // value with two mutators is one whose effective setting depends on which you
    // ask. The identity is the file's alone.
    for key in ["channel.spam_warn_rate", "channel.port", "identity.cert"] {
        let env = s.execute(
            &pinning(
                "record<out: int>",
                &format!("grimm pin_config '{key}' $data.pid 1\n{{ out: 0 }}"),
            ),
            &my_pid(),
        );
        assert!(has_error(&env), "`{key}` must not be pinnable; got {env}");
        assert!(
            error_text(&env).contains("not pinnable"),
            "got {}",
            error_text(&env),
        );
    }
    clear_config_pins();
}

fn a_pin_is_held_to_the_config_files_own_bounds(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    clear_config_pins();
    // A fraction outside (0, 1] would warn always or never. Sharing the file
    // layer's validator is what stops a pin reaching a state a config load would
    // have refused.
    for value in ["0.0", "1.5", "-0.2"] {
        let env = s.execute(
            &pinning(
                "record<out: int>",
                &format!(
                    "grimm pin_config 'supervisor.ram_warn_fraction' $data.pid {value}\n\
                     {{ out: 0 }}",
                ),
            ),
            &my_pid(),
        );
        assert!(has_error(&env), "fraction {value} must be refused; got {env}");
    }
    clear_config_pins();
}

fn a_pin_against_a_dead_process_is_refused(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    clear_config_pins();
    // pid 0 is never a real process. Refusing at pin time rather than reaping a
    // tick later matters because the two look identical a second afterwards.
    let env = s.execute(
        &reading(
            "record<out: int>",
            "grimm pin_config 'supervisor.cpu_warn_fraction' 0 0.5\n{ out: 0 }",
        ),
        "null",
    );
    assert!(has_error(&env), "got {env}");
    assert_eq!(
        error_text(&env).matches("not running").count(),
        1,
        "the refusal should say why, once; got {}",
        error_text(&env),
    );
    let after = s.execute(
        &reading(
            "record<value: float>",
            "{ value: (grimm get_config 'supervisor.cpu_warn_fraction') }",
        ),
        "null",
    );
    assert_eq!(
        after["result"]["value"].as_f64(),
        Some(0.80),
        "a refused pin must leave nothing behind; got {after}",
    );
    clear_config_pins();
}

fn the_config_decls_reach_the_stateful_lane_too(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    let env = s.execute(
        "def --env execute [data: nothing]: nothing -> record<tables: int> { { tables: (grimm get_config_all | columns | length) } }",
        "null",
    );
    assert_eq!(
        env["result"]["tables"].as_i64(),
        Some(3),
        "the whole grimm family registers per eval, on both lanes; got {env}",
    );
}
