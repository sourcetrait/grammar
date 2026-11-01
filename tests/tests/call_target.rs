//! A call runs its rig's committed copy, wherever the host starts.

use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// A main answering where it was read from.
fn which(from: &str) -> String {
    valid_function_source("x: int", "from: string", &format!("{{ from: '{from}' }}"))
}

/// A call serves its commit when the host starts beside the equip's source.
///
/// The host starts in the directory holding the equip's source, at the very
/// path the call's import names; the source is edited after its commit, and the
/// call still runs what was committed.
#[tested]
fn a_call_runs_the_committed_copy_not_the_start_directorys_source() {
    let t = testing::test!({ .using_temp_dir() });
    let cwd = t.temp_dir().join("agent");
    std::fs::create_dir_all(&cwd).expect("mkdir the start directory");
    let mut host = Host::spawn_in(
        &t.temp_dir().join("scratch"),
        &["--id", "target", "--namespace", "default"],
        &cwd,
    );
    let (resp, rig) = host.equip_new("sourcetrait/targetlib", &cwd);
    assert!(!has_error_path(&resp), "equip new should succeed; got {resp}");
    write_source(&rig, "mod.nu", "export module m\n");
    write_source(&rig, "m/mod.nu", "export use which\n");
    write_source(&rig, "m/which/mod.nu", &which("committed"));
    let committed = host.commit("sourcetrait/targetlib");
    assert!(!has_error_path(&committed), "the commit should succeed; got {committed}");

    write_source(&rig, "m/which/mod.nu", &which("source"));
    let called = host.call_np("sourcetrait/targetlib:m:which", "{x: 1}");
    assert_eq!(
        envelope(&called)["result"]["from"].as_str(),
        Some("committed"),
        "a call serves the commit, never the start directory's edit; got {called}",
    );
}
