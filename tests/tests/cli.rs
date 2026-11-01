
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

#[tested]
fn cli_info_prints_its_envelope() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(t.temp_dir(), &["--id", "cid", "cli", "interface", "interface info"]);
    assert!(out.status.success(), "cli interface info should exit 0; got {out:?}");
    let v = stdout_envelope(&out);
    let info = &v["result"];
    assert_eq!(info["name"].as_str(), Some("grammar"), "got {v}");
    assert_eq!(info["id"].as_str(), Some("cid"), "got {v}");
    assert_eq!(info["namespace"].as_str(), Some("default"), "got {v}");
    assert!(
        info["nu_version"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
        "info should carry nu_version; got {v}",
    );
}

#[tested]
fn cli_removes_its_scratch_directories_at_the_end() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(t.temp_dir(), &["--id", "cid", "cli", "interface", "interface info"]);
    assert!(out.status.success(), "cli interface info should exit 0; got {out:?}");
    let v = stdout_envelope(&out);
    for field in ["tmp", "transient"] {
        let dir = v["result"][field].as_str().unwrap_or_default();
        assert!(!dir.is_empty(), "a one-shot run makes its {field} directory too; got {v}");
        assert!(
            !std::path::Path::new(dir).exists(),
            "a one-shot run removes its {field} directory as it ends; {dir} remains",
        );
    }
}

#[tested]
fn cli_stops_each_system_it_started() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(t.temp_dir(), &["--id", "cid", "cli", "interface", "interface info"]);
    assert!(out.status.success(), "cli interface info should exit 0; got {out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("ended unasked"), "a one-shot run stops its systems in order; got {stderr}");
}

#[tested]
fn cli_equip_lifecycle_and_call() {
    let t = testing::test!({ .using_temp_dir() });
    let equip = equip_source(&t.temp_dir().join("src"), "sourcetrait/clilib");
    let src = equip.join("rig");

    let new = format!("interface equip new {}", nu_literal(equip.to_str().unwrap()));
    let established = run_output(t.temp_dir(), &["--id", "cid", "cli", "interface", &new]);
    assert!(
        established.status.success(),
        "cli interface equip new should exit 0; got {established:?}",
    );

    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use double\n");
    write_source(
        &src,
        "m/double/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );

    let committed = run_output(
        t.temp_dir(),
        &["--id", "cid", "cli", "interface", "interface equip commit 'sourcetrait/clilib'"],
    );
    assert!(
        committed.status.success(),
        "cli interface equip commit should exit 0; got stdout={:?} stderr={:?}",
        stdout_str(&committed),
        String::from_utf8_lossy(&committed.stderr),
    );

    let called = run_output(
        t.temp_dir(),
        &["--id", "cid", "cli", "call", "sourcetrait/clilib:m:double", "{x: 21}"],
    );
    assert!(
        called.status.success(),
        "cli call should exit 0; got stdout={:?} stderr={:?}",
        stdout_str(&called),
        String::from_utf8_lossy(&called.stderr),
    );
    let v = stdout_envelope(&called);
    assert_eq!(v["result"]["out"].as_i64(), Some(42), "got {v}");
}

#[tested]
fn cli_execute_evaluates_a_definition_against_nuon_data() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(
        t.temp_dir(),
        &[
            "--id",
            "cid",
            "cli",
            "execute",
            "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x + 1) } }",
            "{x: 5}",
        ],
    );
    assert!(
        out.status.success(),
        "cli execute should exit 0; got stdout={:?} stderr={:?}",
        stdout_str(&out),
        String::from_utf8_lossy(&out.stderr),
    );
    let v = stdout_envelope(&out);
    assert_eq!(v["result"]["out"].as_i64(), Some(6), "got {v}");
    assert!(v["nonce"].as_str().is_some(), "the envelope should carry a nonce; got {v}");
}

#[tested]
fn cli_error_envelope_exits_one() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(
        t.temp_dir(),
        &["--id", "cid", "cli", "call", "sourcetrait/ghost:m:noop", "{}"],
    );
    assert_eq!(out.status.code(), Some(1), "an error envelope should exit 1; got {out:?}");
    let v = stdout_envelope(&out);
    assert!(v.get("error").is_some(), "the error envelope should print; got {v}");
    assert!(v["nonce"].as_str().is_some(), "a failure carries its nonce too; got {v}");
}

#[tested]
fn cli_output_is_one_nuon_record() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(t.temp_dir(), &["--id", "cid", "cli", "interface", "interface info"]);
    assert!(out.status.success(), "cli interface info should exit 0; got {out:?}");
    let text = stdout_str(&out);
    assert!(!text.contains('\u{1b}'), "stdout must carry no ANSI escapes; got {text:?}");
    let trimmed = text.trim();
    assert!(
        trimmed.starts_with("{nonce: ") && trimmed.ends_with('}'),
        "stdout should be exactly one NUON record led by its nonce; got {text:?}",
    );
    assert!(try_nuon(trimmed).is_some(), "and it parses as NUON; got {text:?}");
}

#[tested]
fn cli_process_kill_prints_a_null_result_and_exits_zero() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(
        t.temp_dir(),
        &["--id", "cid", "cli", "interface", "interface process kill 'doesnotexist'"],
    );
    assert!(out.status.success(), "cli interface process kill should exit 0; got {out:?}");
    let v = stdout_envelope(&out);
    assert!(
        v.get("result").is_some_and(|result| result.is_nothing()),
        "a void command answers a null result; got {v}",
    );
}

#[tested]
fn cli_interface_refusal_exits_one() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(
        t.temp_dir(),
        &["--id", "cid", "cli", "interface", "interface info | interface purview list"],
    );
    assert_eq!(out.status.code(), Some(1), "a refused command should exit 1; got {out:?}");
    let v = stdout_envelope(&out);
    assert_eq!(v["error"][0]["kind"].as_str(), Some("interface::invalid"), "got {v}");
    assert!(v["nonce"].as_str().is_some(), "the refusal carries its nonce; got {v}");
}

/// Deny gates an agent's host, never the operator's one-shot path.
///
/// The one-shot path keeps the whole interface, a denied command and a denied
/// tool alike.
#[tested]
fn cli_deny_does_not_reach_the_one_shot_path() {
    let t = testing::test!({ .using_temp_dir() });
    let out = run_output(
        t.temp_dir(),
        &[
            "--id",
            "cid",
            "--deny",
            "interface,purview configure",
            "cli",
            "interface",
            "interface purview configure 'cli/probe' ['*']",
        ],
    );
    assert!(
        out.status.success(),
        "a denied command still runs on the one-shot path; got stdout={:?} stderr={:?}",
        stdout_str(&out),
        String::from_utf8_lossy(&out.stderr),
    );
    let v = stdout_envelope(&out);
    assert!(v["result"]["signatures"].as_str().is_some(), "got {v}");
}
