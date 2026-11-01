//! In-process equip lifecycle (new / uninstall) against the namespace on disk. The
//! FRESH-PROCESS substrate init (keypair + repo creation at startup) is a SYSTEM
//! concern -> `sourcetrait_grammar_tests` (substrate.rs).

use std::path::Path;
use std::process::Command;

use sourcetrait_grammar_mcp::guts::{
    TestServer, equip_source, has_error, valid_function_source, write_source,
};
use sourcetrait_common::testing::prelude::*;

/// One shared in-process server per test binary: constructing a TestServer runs
/// the namespace substrate (keypair, equip repo git config), which must not race
/// itself across parallel tests.
static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

/// Commit-log subjects in the in-process namespace's equip repo (read-only).
fn git_log_subjects(repo: &Path) -> Vec<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("log")
        .arg("--pretty=%s")
        .output()
        .expect("git log");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect()
}

#[tested]
fn equip_new_writes_repo_and_records_meta() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/mylib");
    let src = eq.join("rig");
    let env = s.equip("new", eq.to_str().unwrap());
    assert!(!has_error(&env), "equip(new) should succeed; got {env}");
    let rig_dir = s.rig_dir("sourcetrait/mylib");
    assert!(rig_dir.exists(), "rig dir should exist at {}", rig_dir.display());
    assert!(rig_dir.join("mod.nu").exists(), "rig mod.nu should exist");
    let meta_path = rig_dir.join(".meta/rig.nuon");
    assert!(meta_path.exists(), "the meta sidecar should exist, and be NUON");
    let meta = s.rig_index("sourcetrait/mylib");
    assert_eq!(meta["source_path"].as_str(), src.to_str());
    assert!(
        meta.get("kind").is_none(),
        "meta should not carry a kind field; got {meta}",
    );
    assert!(src.exists(), "source dir should exist");
    assert!(src.join("mod.nu").exists(), "source rig mod.nu should be seeded");
    let log = git_log_subjects(&s.equip_root());
    assert!(
        log.iter().any(|l| l == "new equip sourcetrait/mylib"),
        "expected new-equip commit in log; got {log:?}",
    );
}

#[tested]
fn uninstall_removes_subtree_keeps_source() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let src = s.new_rig(t.temp_dir(), "sourcetrait/droppable");
    write_source(&src, "mod.nu", "export module m\n");
    write_source(&src, "m/mod.nu", "export use thing\n");
    write_source(
        &src,
        "m/thing/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
    let _ = s.commit("sourcetrait/droppable");
    assert!(s.rig_dir("sourcetrait/droppable").exists());
    let env = s.equip("uninstall", src.parent().unwrap().to_str().unwrap());
    assert!(!has_error(&env), "uninstall should succeed; got {env}");
    assert!(
        !s.rig_dir("sourcetrait/droppable").exists(),
        "rig dir should be gone after uninstall",
    );
    assert!(src.exists(), "source should remain after uninstall");
    assert!(
        src.join("m/thing/mod.nu").exists(),
        "source files should remain after uninstall",
    );
    let log = git_log_subjects(&s.equip_root());
    assert!(
        log.iter().any(|l| l == "uninstall equip sourcetrait/droppable"),
        "expected uninstall-equip commit in log; got {log:?}",
    );
}

#[tested]
fn uninstall_missing_succeeds() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();
    let eq = equip_source(t.temp_dir(), "sourcetrait/neverexisted");
    let env = s.equip("uninstall", eq.to_str().unwrap());
    assert!(
        !has_error(&env),
        "uninstall of an unknown equip should be idempotent success; got {env}",
    );
}
