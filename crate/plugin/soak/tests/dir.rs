//! `soak dir` against the failures the nu_plugins copy carried: a crash, a
//! silent loss, or a path resolved in the wrong place, each pinned here.

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::Path;

use nu_plugin_soak::SoakPlugin;
use nu_plugin_test_support::PluginTest;
use nu_protocol::{Span, Value};
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// The fill every case renders with.
const FILL: &str = "{ name: World }";

/// Evaluates `source` with the plugin, the engine's current directory `cwd`.
fn eval(cwd: &Path, source: &str) -> Result<Value, String> {
    let mut test = PluginTest::new("soak", SoakPlugin.into()).map_err(|e| e.to_string())?;
    test.engine_state_mut()
        .add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));
    let data = test.eval(source).map_err(|e| e.to_string())?;
    data.into_value(Span::test_data()).map_err(|e| e.to_string())
}

/// The `soak dir` call rendering `src` into `dst` with the fill.
fn soak_dir(src: &Path, dst: &Path) -> String {
    format!("{FILL} | soak dir '{}' '{}'", src.display(), dst.display())
}

/// Writes `content` at `path`, its parents made.
fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("make the parent");
    fs::write(path, content).expect("write the fixture file");
}

/// The file at `path`, or nothing when it cannot be read.
fn read(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

/// A partials directory, so a case reaches past the partials crash.
fn partials(src: &Path) {
    write(&src.join("liquid").join("part.liquid"), "part");
}

/// soak dir renders in place: what it writes is overwritten, every other
/// file in the destination stays, and dotted entries and `liquid/` are not
/// emitted.
#[tested]
fn renders_in_place_keeping_the_rest() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let (src, dst) = (root.join("src"), root.join("dst"));
    partials(&src);
    write(&src.join("one.txt.liquid"), "new {{ name }}");
    write(&src.join(".env.liquid"), "secret");
    write(&src.join(".hidden").join("x.txt"), "x");
    write(&dst.join("one.txt"), "old");
    write(&dst.join("stale.txt"), "stale");

    let result = eval(root, &soak_dir(&src, &dst));
    assert!(result.is_ok(), "the soak renders; got {result:?}");
    assert_eq!(read(&dst.join("one.txt")).as_deref(), Some("new World"));
    assert_eq!(read(&dst.join("stale.txt")).as_deref(), Some("stale"));
    assert!(!dst.join(".env").exists(), "a dotted template is skipped");
    assert!(!dst.join(".hidden").exists(), "a dotted directory is skipped");
    assert!(!dst.join("liquid").exists(), "the partials are never emitted");
}

/// A missing source is an error naming it, and nothing is written.
#[tested]
fn a_missing_source_is_an_error_naming_it() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let (src, dst) = (root.join("absent"), root.join("dst"));

    let error = eval(root, &soak_dir(&src, &dst)).expect_err("a missing source fails the soak");
    assert!(error.contains("absent"), "the error names the source; got {error}");
    assert!(!dst.exists(), "nothing is written");
}

/// A source with no `liquid/` partials directory renders.
#[tested]
fn a_source_without_partials_renders() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let (src, dst) = (root.join("src"), root.join("dst"));
    write(&src.join("hello.txt.liquid"), "Hello, {{ name }}!");

    let result = eval(root, &soak_dir(&src, &dst));
    assert!(result.is_ok(), "the soak renders without partials; got {result:?}");
    assert_eq!(read(&dst.join("hello.txt")).as_deref(), Some("Hello, World!"));
}

/// A template that fails to parse is an error naming it, and nothing is
/// written: the destination is never made.
#[tested]
fn a_template_error_writes_nothing() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let (src, dst) = (root.join("src"), root.join("dst"));
    partials(&src);
    write(&src.join("a_good.txt"), "good");
    write(&src.join("b_bad.txt.liquid"), "{{ unclosed");

    let error = eval(root, &soak_dir(&src, &dst)).expect_err("a syntax error fails the soak");
    assert!(error.contains("b_bad.txt.liquid"), "the error names the template; got {error}");
    assert!(!dst.exists(), "nothing is written, the destination never made");
}

/// `src` and `dst` resolve against the caller's directory.
#[tested]
fn relative_paths_resolve_against_the_callers_directory() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let src = root.join("tmpl_src");
    partials(&src);
    write(&src.join("hello.txt.liquid"), "Hello, {{ name }}!");

    let result = eval(root, &format!("{FILL} | soak dir tmpl_src tmpl_out"));
    assert!(result.is_ok(), "the paths resolve in the caller's directory; got {result:?}");
    assert_eq!(read(&root.join("tmpl_out").join("hello.txt")).as_deref(), Some("Hello, World!"));
}

/// A source under a dotted directory renders: dottedness is judged by the
/// names below the source.
#[tested]
fn a_source_under_a_dotted_directory_renders() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let (src, dst) = (root.join(".hidden").join("src"), root.join("dst"));
    partials(&src);
    write(&src.join("hello.txt.liquid"), "Hello, {{ name }}!");

    let result = eval(root, &soak_dir(&src, &dst));
    assert!(result.is_ok(), "the soak renders; got {result:?}");
    assert_eq!(read(&dst.join("hello.txt")).as_deref(), Some("Hello, World!"));
}

/// Every entry arrives, more than 10,000 of them, and a file deeper than 100
/// directories.
#[tested]
fn every_entry_arrives_however_many_and_however_deep() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let (src, dst) = (root.join("src"), root.join("dst"));
    partials(&src);
    for i in 0..10_001 {
        write(&src.join("many").join(format!("{i}.txt")), "x");
    }
    let deep = (0..101).fold(Path::new("deep").to_path_buf(), |path, _| path.join("d"));
    write(&src.join(&deep).join("bottom.txt"), "bottom");

    let result = eval(root, &soak_dir(&src, &dst));
    assert!(result.is_ok(), "the soak renders; got {result:?}");
    let many = fs::read_dir(dst.join("many")).map(|entries| entries.count()).unwrap_or(0);
    assert_eq!(many, 10_001, "every file arrives");
    assert_eq!(read(&dst.join(&deep).join("bottom.txt")).as_deref(), Some("bottom"));
}

/// Symlinks in the source are followed, a file's and a directory's. Unix
/// alone: on Windows a symlink takes a privilege a plain session lacks.
#[cfg(unix)]
#[tested]
fn symlinks_in_the_source_are_followed() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let (src, dst) = (root.join("src"), root.join("dst"));
    partials(&src);
    write(&root.join("outside.txt.liquid"), "Hello, {{ name }}!");
    write(&root.join("shared").join("inner.txt"), "inner");
    symlink(root.join("outside.txt.liquid"), src.join("linked.txt.liquid")).expect("link a file");
    symlink(root.join("shared"), src.join("linkdir")).expect("link a directory");

    let result = eval(root, &soak_dir(&src, &dst));
    assert!(result.is_ok(), "the soak renders; got {result:?}");
    assert_eq!(read(&dst.join("linked.txt")).as_deref(), Some("Hello, World!"));
    assert_eq!(read(&dst.join("linkdir").join("inner.txt")).as_deref(), Some("inner"));
}

/// A symlink loop in the source is refused, and nothing is written. Unix
/// alone, as symlinks are.
#[cfg(unix)]
#[tested]
fn a_symlink_loop_is_refused() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let (src, dst) = (root.join("src"), root.join("dst"));
    partials(&src);
    write(&src.join("hello.txt.liquid"), "Hello, {{ name }}!");
    symlink(&src, src.join("loop")).expect("link the source into itself");

    let result = eval(root, &soak_dir(&src, &dst));
    assert!(result.is_err(), "a loop fails the soak; got {result:?}");
    assert!(!dst.exists(), "nothing is written");
}

/// Two sources writing one output are refused, and nothing is written.
#[tested]
fn two_sources_for_one_output_are_refused() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let (src, dst) = (root.join("src"), root.join("dst"));
    partials(&src);
    write(&src.join("a.txt"), "plain");
    write(&src.join("a.txt.liquid"), "templated");

    let error = eval(root, &soak_dir(&src, &dst)).expect_err("a collision fails the soak");
    assert!(error.contains("a.txt"), "the error names the output; got {error}");
    assert!(!dst.exists(), "nothing is written");
}

/// A destination inside the source is refused.
#[tested]
fn a_destination_inside_the_source_is_refused() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let src = root.join("src");
    partials(&src);
    write(&src.join("hello.txt.liquid"), "Hello, {{ name }}!");

    let result = eval(root, &soak_dir(&src, &src.join("out")));
    assert!(result.is_err(), "the soak refuses; got {result:?}");
    assert!(!src.join("out").exists(), "nothing is written");
}

/// An output that would land inside the source is refused, the source left
/// as it was.
#[tested]
fn an_output_inside_the_source_is_refused() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let src = root.join("t");
    partials(&src);
    write(&src.join("x.txt"), "source");
    write(&src.join("t").join("x.txt"), "overwrite");

    let result = eval(root, &soak_dir(&src, root));
    assert!(result.is_err(), "the soak refuses; got {result:?}");
    assert_eq!(read(&src.join("x.txt")).as_deref(), Some("source"));
}

/// A write's error surfaces: an output linked to `/dev/full` fails the soak.
/// Unix alone, which has the device.
#[cfg(unix)]
#[tested]
fn a_write_error_surfaces() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let (src, dst) = (root.join("src"), root.join("dst"));
    partials(&src);
    write(&src.join("hello.txt.liquid"), "Hello, {{ name }}!");
    fs::create_dir_all(&dst).expect("make the destination");
    symlink("/dev/full", dst.join("hello.txt")).expect("link the output to a full device");

    let result = eval(root, &soak_dir(&src, &dst));
    assert!(result.is_err(), "the full device's error fails the soak; got {result:?}");
}
