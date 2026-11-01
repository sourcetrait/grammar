//! grammar-ssh's command line: `serve` alone, its id and namespace required.

use std::process::Command;

#[test]
fn serve_takes_its_id_and_namespace_by_name() {
    let cases: [&[&str]; 3] = [&["serve"], &["serve", "--id", "fixture"], &["serve", "--namespace", "default"]];
    for args in cases {
        let out = Command::new(env!("CARGO_BIN_EXE_grammar-ssh")).args(args).output().expect("grammar-ssh runs");
        assert_eq!(out.status.code(), Some(2), "a usage error: {out:?}");
        assert!(out.stdout.is_empty(), "nothing on stdout, the wire's");
    }
}

#[test]
fn the_workdir_is_optional() {
    let out = Command::new(env!("CARGO_BIN_EXE_grammar-ssh")).args(["serve", "--help"]).output().expect("grammar-ssh runs");
    assert!(out.status.success(), "{out:?}");
    let help = String::from_utf8_lossy(&out.stdout);
    let usage = help.lines().find(|line| line.starts_with("Usage:")).expect("a usage line");
    assert!(usage.contains("--id <ID> --namespace <NAMESPACE>"), "the two required flags: {usage}");
    assert!(!usage.contains("--workdir"), "the workdir isn't required: {usage}");
    assert!(help.contains("--workdir <WORKDIR>"), "{help}");
}

#[test]
fn an_unknown_command_is_a_usage_error() {
    for command in ["instance", "bridge", "cli"] {
        let out = Command::new(env!("CARGO_BIN_EXE_grammar-ssh")).arg(command).output().expect("grammar-ssh runs");
        assert_eq!(out.status.code(), Some(2), "{command}: {out:?}");
    }
}
