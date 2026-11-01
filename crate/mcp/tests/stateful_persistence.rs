
use sourcetrait_grammar_mcp::guts::{TestServer, has_error, nuon};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary (substrate init must not race
/// itself). Every case below asserts CROSS-CALL state on the one serial stateful
/// engine, so the whole file runs as one Stepper-sequenced test: interleaved
/// env/cd mutations from parallel siblings would break the persistence asserts.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .setup(|_| TestServer::new())
});

#[tested]
fn stateful_persistence_is_sequenced() {
    testing::Stepper::builder("stateful_persistence")
        .init(|_t, _o: ()| testing::StepState((), ()))
        .step("env_mutation_persists_across_stateful_calls", |t, s, p| {
            env_mutation_persists_across_stateful_calls(t);
            testing::StepState(s, p)
        })
        .step("cd_persists_across_stateful_calls", |t, s, p| {
            cd_persists_across_stateful_calls(t);
            testing::StepState(s, p)
        })
        .step("an_inner_def_does_not_persist_across_stateful_calls", |t, s, p| {
            an_inner_def_does_not_persist_across_stateful_calls(t);
            testing::StepState(s, p)
        })
        .step("stateful_state_does_not_leak_into_the_pool", |t, s, p| {
            stateful_state_does_not_leak_into_the_pool(t);
            testing::StepState(s, p)
        })
        .step("a_stateful_cd_does_not_move_the_host_process_cwd", |t, s, p| {
            a_stateful_cd_does_not_move_the_host_process_cwd(t);
            testing::StepState(s, p)
        })
        .step("a_multi_line_definition_with_a_command_then_a_record_parses", |t, s, p| {
            a_multi_line_definition_with_a_command_then_a_record_parses(t);
            testing::StepState(s, p)
        })
        .finalize()
        .run(TESTING.as_testable(), ());
}

fn env_mutation_persists_across_stateful_calls(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    let first = s.execute(
        "def --env execute [data: record<value: string>]: nothing -> record<wrote: string> {\n    $env.SHOT_DEMO = $data.value\n    { wrote: $data.value }\n}",
        "{value: alpha}",
    );
    assert_eq!(first["result"], nuon("{wrote: alpha}"), "got {first}");

    let second = s.execute(
        "def --env execute [data: nothing]: nothing -> record<saw: string> { { saw: $env.SHOT_DEMO } }",
        "null",
    );
    assert_eq!(
        second["result"],
        nuon("{saw: alpha}"),
        "the env mutation from call 1 should persist; got {second}",
    );
}

fn cd_persists_across_stateful_calls(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    let first = s.execute(
        "def --env execute [data: record<target: string>]: nothing -> record<cwd: string> {\n    cd $data.target\n    { cwd: (pwd) }\n}",
        "{target: /tmp}",
    );
    assert!(!has_error(&first), "got {first}");
    let second = s.execute(
        "def --env execute [data: nothing]: nothing -> record<cwd: string> { { cwd: $env.PWD } }",
        "null",
    );
    assert_eq!(second["result"], nuon(r#"{cwd: "/tmp"}"#), "cd should propagate; got {second}");
}

/// env and `cd` carry over, which is the stateful lane's purpose; a def made inside
/// `execute` never does: it is scoped to that call, visible within it, gone from the
/// next.
fn an_inner_def_does_not_persist_across_stateful_calls(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    let first = s.execute(
        "def --env execute [data: nothing]: nothing -> record<seen: bool> {\n    def stateful_scoped_probe [] { 1 }\n    { seen: ('stateful_scoped_probe' in (scope commands | get name)) }\n}",
        "null",
    );
    assert_eq!(
        first["result"],
        nuon("{seen: true}"),
        "the def should be visible within the call that made it; got {first}",
    );

    let second = s.execute(
        "def --env execute [data: nothing]: nothing -> record<seen: bool> { { seen: ('stateful_scoped_probe' in (scope commands | get name)) } }",
        "null",
    );
    assert_eq!(
        second["result"],
        nuon("{seen: false}"),
        "a def must not persist into the next stateful call; got {second}",
    );
}

fn stateful_state_does_not_leak_into_the_pool(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    let set = s.execute(
        "def --env execute [data: nothing]: nothing -> record<ok: bool> {\n    $env.STATEFUL_ONLY = 'kept'\n    { ok: true }\n}",
        "null",
    );
    assert!(!has_error(&set), "got {set}");
    let env = s.execute(
        "def execute [data: nothing]: nothing -> record<seen: bool> { { seen: ('STATEFUL_ONLY' in $env) } }",
        "null",
    );
    assert_eq!(
        env["result"],
        nuon("{seen: false}"),
        "the stateless pool must NOT see the stateful lane's env; got {env}",
    );
}

/// nushell's `EngineState::merge_env` ends by chdir'ing the process to `$env.PWD`.
/// In-process that would move the whole grammar host for the rest of its life, so the
/// stateful lane merges env WITHOUT that sync (the engine's merge_env_no_chdir) and
/// `cd` stays engine state.
fn a_stateful_cd_does_not_move_the_host_process_cwd(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    let before = std::env::current_dir().expect("read the process cwd");
    let env = s.execute(
        "def --env execute [data: record<dir: string>]: nothing -> record<pwd: string> {\n    cd $data.dir\n    { pwd: $env.PWD }\n}",
        "{dir: /etc}",
    );
    assert_eq!(
        env["result"],
        nuon(r#"{pwd: "/etc"}"#),
        "cd should still set $env.PWD; got {env}",
    );
    let after = std::env::current_dir().expect("read the process cwd");
    assert_eq!(
        before, after,
        "a stateful `cd` must not move the host process cwd - in-process there is no \
         worker subprocess to absorb it",
    );
}

fn a_multi_line_definition_with_a_command_then_a_record_parses(_t: &testing::Testable<'_, '_, '_>) {
    let s = TESTING.harness();
    let env = s.execute(
        "def --env execute [data: record<x: int>]: nothing -> record<y: int, slept_ms: int> {\n    sleep 50ms\n    let doubled = ($data.x * 2)\n    { y: $doubled, slept_ms: 50 }\n}",
        "{x: 7}",
    );
    assert_eq!(env["result"], nuon("{y: 14, slept_ms: 50}"), "got {env}");
}
