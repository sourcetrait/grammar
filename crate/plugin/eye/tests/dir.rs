//! `eye dir` through the plugin: the render, ignore and regard, the `.git`
//! default, the help text's example, the glob forms, and a relative `dir`.

use std::fs;
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

/// The tree `eye dir` renders for `dir`, `globs` its ignore and regard lists.
fn tree(dir: &Path, globs: &str) -> String {
    let value = eval(dir, &format!("eye dir '{}' {globs}", dir.display())).expect("eye dir renders");
    value.as_str().expect("the tree is a string").to_string()
}

/// Writes a fixture file of `bytes` bytes, its parents made.
fn write(path: &Path, bytes: usize) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("make the parent");
    fs::write(path, vec![b'x'; bytes]).expect("write the fixture file");
}

/// Files come before directories, each with its size; symlinks show their
/// targets; `.git` is ignored by default.
#[tested]
fn renders_files_before_dirs_with_sizes_symlinks_and_default_git_ignore() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    write(&root.join("a.txt"), 0);
    write(&root.join("b.txt"), 2048);
    write(&root.join("sp ace.txt"), 5);
    write(&root.join(".git").join("config"), 10);
    fs::create_dir(root.join("Zed")).unwrap();
    write(&root.join("apple").join("inner.txt"), 40);
    fs::create_dir(root.join("empty")).unwrap();
    symlink("a.txt", root.join("link_f")).unwrap();
    symlink("apple", root.join("link_d")).unwrap();
    symlink("nowhere", root.join("link_broken")).unwrap();

    let r = root.display().to_string();
    let expected = [
        format!("{r}/"),
        " a.txt 0".to_string(),
        " b.txt 2kb".to_string(),
        " link_broken -> nowhere".to_string(),
        format!(" link_d -> {r}/apple/"),
        format!(" link_f -> {r}/a.txt 0"),
        " `sp ace.txt` 5b".to_string(),
        " Zed/".to_string(),
        " apple/".to_string(),
        "  inner.txt 40b".to_string(),
        " empty/".to_string(),
    ]
    .join("\n")
        + "\n";

    assert_eq!(tree(root, "[] []"), expected);
}

/// An ignore glob denies and a regard glob rescues.
#[tested]
fn ignore_denies_and_regard_rescues() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    write(&root.join("keep.md"), 3);
    write(&root.join("skip.txt"), 3);
    write(&root.join("also.txt"), 3);

    let denied = tree(root, "['*.txt'] []");
    assert!(denied.contains("keep.md"));
    assert!(!denied.contains("skip.txt"));
    assert!(!denied.contains("also.txt"));

    let rescued = tree(root, "['*.txt'] ['also.txt']");
    assert!(rescued.contains("also.txt"));
    assert!(!rescued.contains("skip.txt"));
}

/// `.git` is ignored by default and a regard glob renders it and its children.
#[tested]
fn git_default_ignored_but_regardable() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    write(&root.join(".git").join("config"), 4);
    write(&root.join("real.txt"), 4);

    let default = tree(root, "[] []");
    assert!(!default.contains(".git"));
    assert!(default.contains("real.txt"));

    let regarded = tree(root, "[] ['**/.git']");
    assert!(regarded.contains(".git/"));
    assert!(regarded.contains("config"));
}

/// The help text's example is a true golden: its tree renders byte for byte.
#[tested]
fn renders_the_help_example() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    write(&root.join(".gitignore"), 0);
    let subdir = root.join("subdir");
    fs::create_dir(&subdir).unwrap();
    fs::File::create(subdir.join("file1.txt"))
        .unwrap()
        .set_len(32 * 1024 * 1024)
        .unwrap();
    fs::create_dir(subdir.join("otherdir")).unwrap();
    write(&subdir.join("somedir").join(".file2"), 40);

    let r = root.display().to_string();
    let expected = [
        format!("{r}/"),
        " .gitignore 0".to_string(),
        " subdir/".to_string(),
        "  file1.txt 32mb".to_string(),
        "  otherdir/".to_string(),
        "  somedir/".to_string(),
        "   .file2 40b".to_string(),
    ]
    .join("\n")
        + "\n";

    assert_eq!(tree(root, "[] []"), expected);
}

/// Globs match the paths below the directory: `target` the top one alone,
/// `**/target` every one, `target/**` the top one's contents with its line
/// kept, and a trailing slash nothing.
#[tested]
fn the_glob_forms_match_paths_below_the_directory() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    write(&root.join("target").join("top.o"), 1);
    write(&root.join("sub").join("target").join("nested.o"), 1);

    let top = tree(root, "['target'] []");
    assert!(!top.contains("\n target/") && !top.contains("top.o"), "the top target goes:\n{top}");
    assert!(top.contains("nested.o"), "a nested target stays:\n{top}");

    let every = tree(root, "['**/target'] []");
    assert!(!every.contains("top.o") && !every.contains("nested.o"), "every target goes:\n{every}");

    let contents = tree(root, "['target/**'] []");
    assert!(contents.contains("\n target/\n"), "the top target's line stays:\n{contents}");
    assert!(!contents.contains("top.o"), "its contents go:\n{contents}");
    assert!(contents.contains("nested.o"), "a nested target stays:\n{contents}");

    let slash = tree(root, "['target/' '**/target/'] []");
    assert!(
        slash.contains("top.o") && slash.contains("nested.o"),
        "a trailing slash matches nothing:\n{slash}",
    );
}

/// A relative `dir` resolves against the caller's directory.
#[tested]
fn a_relative_dir_resolves_against_the_callers_directory() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    write(&root.join("listed").join("here.txt"), 1);

    let rendered = eval(root, "eye dir listed").expect("eye dir renders");
    let rendered = rendered.as_str().expect("the tree is a string");
    assert!(rendered.starts_with(&format!("{}/\n", root.join("listed").display())), "got:\n{rendered}");
    assert!(rendered.contains("here.txt 1b"), "got:\n{rendered}");
}
