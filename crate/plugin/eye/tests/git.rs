//! `eye git` through the plugin: `.git` and what `.gitignore` files ignore
//! skipped, nested files and negations, git's scope, and the shared render.

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::Path;

use nu_plugin_eye::EyePlugin;
use nu_plugin_test_support::PluginTest;
use nu_protocol::{Span, Value};
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// Evaluates `source` with the plugin, the engine's current directory `cwd`.
fn eval(cwd: &Path, source: &str) -> Result<Value, String> {
    let mut test = PluginTest::new("eye", EyePlugin.into()).map_err(|e| e.to_string())?;
    test.engine_state_mut()
        .add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));
    let data = test.eval(source).map_err(|e| e.to_string())?;
    data.into_value(Span::test_data()).map_err(|e| e.to_string())
}

/// The tree `eye <command>` renders for `dir`.
fn tree(command: &str, dir: &Path) -> String {
    let value = eval(dir, &format!("eye {command} '{}'", dir.display())).expect("the tree renders");
    value.as_str().expect("the tree is a string").to_string()
}

/// Writes `content` at `path`, its parents made.
fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("make the parent");
    fs::write(path, content).expect("write the fixture file");
}

/// Makes `root` a repository, as git marks one, with its `.git`.
fn repository(root: &Path) {
    write(&root.join(".git").join("HEAD"), "ref: refs/heads/main\n");
}

/// `.git` and what the `.gitignore` files ignore are skipped: a nested file
/// applies below its own directory, and its negation re-includes what a
/// parent's pattern ignored.
#[tested]
fn skips_dot_git_and_what_gitignore_files_ignore() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    repository(root);
    let top_rules = "target/\n*.log\n";
    let sub_rules = "!keep.log\nsecret\n";
    write(&root.join(".gitignore"), top_rules);
    write(&root.join("keep.txt"), "k");
    write(&root.join("a.log"), "l");
    write(&root.join("target").join("x.o"), "o");
    write(&root.join("sub").join(".gitignore"), sub_rules);
    write(&root.join("sub").join("keep.log"), "k");
    write(&root.join("sub").join("secret"), "s");
    write(&root.join("sub").join("visible.txt"), "v");
    write(&root.join("other").join("secret"), "s");

    let r = root.display().to_string();
    let expected = [
        format!("{r}/"),
        format!(" .gitignore {}b", top_rules.len()),
        " keep.txt 1b".to_string(),
        " other/".to_string(),
        "  secret 1b".to_string(),
        " sub/".to_string(),
        format!("  .gitignore {}b", sub_rules.len()),
        "  keep.log 1b".to_string(),
        "  visible.txt 1b".to_string(),
    ]
    .join("\n")
        + "\n";

    assert_eq!(tree("git", root), expected);
}

/// Outside a repository no `.gitignore` applies, as git applies none there.
#[tested]
fn outside_a_repository_no_gitignore_applies() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    write(&root.join(".gitignore"), "*.log\n");
    write(&root.join("a.log"), "l");

    let rendered = tree("git", root);
    assert!(rendered.contains(" a.log 1b"), "got:\n{rendered}");
}

/// A repository's subdirectory takes the `.gitignore` files above it.
#[tested]
fn a_subdirectory_takes_the_gitignore_files_above_it() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    repository(root);
    write(&root.join(".gitignore"), "*.log\n");
    write(&root.join("sub").join("a.log"), "l");
    write(&root.join("sub").join("b.txt"), "b");

    let rendered = tree("git", &root.join("sub"));
    assert!(rendered.contains(" b.txt 1b"), "got:\n{rendered}");
    assert!(!rendered.contains("a.log"), "got:\n{rendered}");
}

/// With nothing ignored, `eye git` renders exactly what `eye dir` renders, the
/// symlinks on Unix alone: on Windows a symlink takes a privilege a plain
/// session lacks.
#[tested]
fn renders_as_eye_dir_renders_when_nothing_is_ignored() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    repository(root);
    write(&root.join("a.txt"), "");
    write(&root.join("sp ace.txt"), "space");
    write(&root.join("apple").join("inner.txt"), "inner");
    fs::create_dir(root.join("empty")).unwrap();
    #[cfg(unix)]
    {
        symlink("a.txt", root.join("link_f")).unwrap();
        symlink("apple", root.join("link_d")).unwrap();
        symlink("nowhere", root.join("link_broken")).unwrap();
    }

    assert_eq!(tree("git", root), tree("dir", root));
}

/// A relative `dir` resolves against the caller's directory.
#[tested]
fn a_relative_dir_resolves_against_the_callers_directory() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    write(&root.join("listed").join("here.txt"), "h");

    let rendered = eval(root, "eye git listed").expect("eye git renders");
    let rendered = rendered.as_str().expect("the tree is a string");
    assert!(rendered.starts_with(&format!("{}/\n", root.join("listed").display())), "got:\n{rendered}");
    assert!(rendered.contains("here.txt 1b"), "got:\n{rendered}");
}
