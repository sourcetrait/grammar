//! grammar-ssh's command line: a bare invocation serves, its flags the MCP's.

use std::process::Command;

#[test]
fn its_flags_are_the_mcps_id_namespace_and_workdir() {
    let out = Command::new(env!("CARGO_BIN_EXE_grammar-ssh")).arg("--help").output().expect("grammar-ssh runs");
    assert!(out.status.success(), "{out:?}");
    let help = String::from_utf8_lossy(&out.stdout);
    let usage = help.lines().find(|line| line.starts_with("Usage:")).expect("a usage line");
    assert!(!usage.contains("<COMMAND>"), "a bare invocation serves: {usage}");
    for flag in ["--id <ID>", "--namespace <NAMESPACE>", "--workdir <WORKDIR>"] {
        assert!(help.contains(flag), "{flag}: {help}");
    }
    let flat = help.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("`default` [default: default]"), "the namespace's default: {help}");
}

#[test]
fn an_argument_it_takes_none_of_is_a_usage_error() {
    for args in [&["serve"][..], &["--test"][..], &["--namespace"][..]] {
        let out = Command::new(env!("CARGO_BIN_EXE_grammar-ssh")).args(args).output().expect("grammar-ssh runs");
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
        assert!(out.stdout.is_empty(), "nothing on stdout, the wire's");
    }
}
