//! The builtins' install: a namespace made at a host's startup gets each builtin
//! its deploy holds, and their purviews; an existing namespace is never touched.

use std::path::{Path, PathBuf};

use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

const EQUIP_DIR_ENV: &str = "GRAMMAR_MCP_EQUIP_DIR";

/// A new namespace gets both builtins and their purviews; `default` stays `*`.
#[tested]
fn a_new_namespace_installs_the_builtins() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let deployed = deploy(root, &["gram", "grammar"]);
    let mut host = spawn(root, &deployed);

    assert_eq!(ping(&mut host, "gram"), Some(7), "gram is installed");
    assert_eq!(ping(&mut host, "grammar"), Some(7), "grammar is installed");
    let rows = purview_rows(&mut host);
    assert_eq!(patterns(&rows, "default"), Some(strings(&["*"])));
    assert_eq!(patterns(&rows, "core"), Some(strings(&["@gram", "@grammar"])));
    assert_eq!(patterns(&rows, "gram"), Some(strings(&["gram:"])));
    assert_eq!(patterns(&rows, "grammar"), Some(strings(&["grammar:"])));
}

/// A builtin the deploy lacks is skipped, and `core` names the one it holds.
#[tested]
fn a_builtin_the_deploy_lacks_is_skipped() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let deployed = deploy(root, &["grammar"]);
    let mut host = spawn(root, &deployed);

    assert_eq!(ping(&mut host, "grammar"), Some(7), "grammar is installed");
    assert_eq!(ping(&mut host, "gram"), None, "gram is not");
    let rows = purview_rows(&mut host);
    assert_eq!(patterns(&rows, "core"), Some(strings(&["@grammar"])));
    assert_eq!(patterns(&rows, "gram"), None);
}

/// A namespace made before is left alone, though its deploy now holds both.
#[tested]
fn an_existing_namespace_is_left_alone() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let empty = root.join("empty");
    std::fs::create_dir_all(&empty).expect("mkdir an empty equip dir");
    drop(spawn(root, &empty));

    let deployed = deploy(root, &["gram", "grammar"]);
    let mut host = spawn(root, &deployed);
    assert_eq!(ping(&mut host, "gram"), None, "nothing installs into an existing namespace");
    assert_eq!(ping(&mut host, "grammar"), None, "nothing installs into an existing namespace");
    let rows = purview_rows(&mut host);
    assert_eq!(patterns(&rows, "core"), None);
}

/// An uninstalled builtin takes its purview and its place in `core`, and stays
/// uninstalled across a restart.
#[tested]
fn an_uninstalled_builtin_stays_uninstalled() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let deployed = deploy(root, &["gram", "grammar"]);
    let mut host = spawn(root, &deployed);
    let source = deployed.join("sourcetrait").join("gram");
    let removed = host.equip("uninstall", source.to_str().expect("utf-8 path"));
    assert!(!has_error_path(&removed), "the uninstall succeeds; got {removed}");
    let rows = purview_rows(&mut host);
    assert_eq!(patterns(&rows, "gram"), None, "gram's purview went with it");
    assert_eq!(patterns(&rows, "core"), Some(strings(&["@grammar"])), "and its place in core");
    drop(host);

    let mut host = spawn(root, &deployed);
    assert_eq!(ping(&mut host, "gram"), None, "a restart does not install it again");
    assert_eq!(ping(&mut host, "grammar"), Some(7), "the other builtin stays");
}

/// A builtin installed by hand seeds its purview and `core`, and brings `@core`
/// into `default`.
#[tested]
fn a_hand_installed_builtin_seeds_its_purviews() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let empty = root.join("empty");
    std::fs::create_dir_all(&empty).expect("mkdir an empty equip dir");
    let mut host = spawn(root, &empty);

    let deployed = deploy(root, &["gram"]);
    let source = deployed.join("sourcetrait").join("gram");
    let installed = host.equip("install", source.to_str().expect("utf-8 path"));
    assert!(!has_error_path(&installed), "the install succeeds; got {installed}");
    assert_eq!(ping(&mut host, "gram"), Some(7), "gram is installed");
    let rows = purview_rows(&mut host);
    assert_eq!(patterns(&rows, "gram"), Some(strings(&["gram:"])));
    assert_eq!(patterns(&rows, "core"), Some(strings(&["@gram"])));
    assert_eq!(patterns(&rows, "default"), Some(strings(&["*", "@core"])));
}

/// A host on `root`'s one namespace, its deploy's equip directory `equip_dir`.
fn spawn(root: &Path, equip_dir: &Path) -> Host {
    Host::spawn_full(
        &root.join("scratch"),
        &["--id", "builtin", "--namespace", "default"],
        &[(EQUIP_DIR_ENV, equip_dir.to_str().expect("utf-8 path"))],
    )
}

/// A deploy's equip directory holding a fixture rig for each builtin named.
fn deploy(root: &Path, names: &[&str]) -> PathBuf {
    let dir = root.join("deploy").join("equip");
    for name in names {
        let rig = dir.join("sourcetrait").join(name).join("rig");
        write_source(&rig, "mod.nu", &format!("# the {name} fixture\nexport module m\n"));
        write_source(&rig, "m/mod.nu", "export module ping\nexport use ping\n");
        write_source(
            &rig,
            "m/ping/mod.nu",
            &valid_function_source("x: int", "out: int", "{ out: $data.x }"),
        );
    }
    dir
}

/// A builtin's ping by its bare name; None when the call fails.
fn ping(host: &mut Host, name: &str) -> Option<i64> {
    let resp = host.call_np(&format!("{name}:m:ping"), "{x: 7}");
    if has_error_path(&resp) {
        return None;
    }
    envelope(&resp)["result"]["out"].as_i64()
}

/// The namespace's purviews as `(id, patterns)` pairs.
fn purview_rows(host: &mut Host) -> Vec<(String, Vec<String>)> {
    let resp = host.interface("interface purview list");
    envelope(&resp)["result"]["purviews"]
        .as_list()
        .expect("the purview list holds the table")
        .iter()
        .map(|row| {
            let id = row[0].as_str().expect("a purview id").to_string();
            let values = row[1]
                .as_list()
                .expect("its patterns")
                .iter()
                .map(|value| value.as_str().expect("a pattern").to_string())
                .collect();
            (id, values)
        })
        .collect()
}

/// One purview's patterns, None when it has no row.
fn patterns(rows: &[(String, Vec<String>)], id: &str) -> Option<Vec<String>> {
    rows.iter().find(|(row, _)| row == id).map(|(_, values)| values.clone())
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}
