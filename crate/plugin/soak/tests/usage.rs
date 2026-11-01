use std::{path::PathBuf, sync::LazyLock};
use nu_plugin_soak::SoakPlugin;
use nu_plugin_test_support::PluginTest;
use nu_protocol::Value;
use sourcetrait_common::tooling;

const SPAN: nu_protocol::Span = nu_protocol::Span::test_data();
static HELLO: LazyLock<Value> =LazyLock::new(|| Value::test_string("Hello, World!"));

fn plug() -> PluginTest {
    PluginTest::new("soak", SoakPlugin.into()).unwrap()
}

fn fixture_dir(subdir: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testing")
        .join(subdir)
        .canonicalize()
        .expect("fixture dir exists")
}

#[test]
fn test_str_soak() {
    let actual = plug()
        .eval("'Hello, {{name}}!' | str soak { name: World }")
        .unwrap()
        .into_value(SPAN)
        .unwrap();

    assert_eq!(*HELLO, actual);
}

#[test]
fn test_soak_str() {
    let actual = plug()
        .eval("{ name: World } | soak str 'Hello, {{name}}!'")
        .unwrap()
        .into_value(SPAN)
        .unwrap();

    assert_eq!(*HELLO, actual);
}

#[test]
fn test_soak_dir() {
    let in_dir = fixture_dir("soak_dir");
    let tempdir = tempfile::tempdir().unwrap();
    let out_dir = tempdir.path();

    let actual = plug()
        .eval(&format!(
            "{{ name: World, color: salmon, impartial: 'REDACTED' }} | soak dir {in_dir} {out_dir}",
            in_dir = in_dir.to_str().unwrap(),
            out_dir = out_dir.to_str().unwrap(),
        ))
        .unwrap()
        .into_value(SPAN)
        .unwrap();

    assert_eq!(Value::test_nothing(), actual);
    let expected_dir = fixture_dir("soaked_dir");
    assert_eq!(None, tooling::path_diff(out_dir, expected_dir).unwrap());
}