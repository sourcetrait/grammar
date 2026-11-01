//! grammard's command line: `serve` alone, its namespace and workdir required.

use std::process::Command;

#[test]
fn serve_takes_its_namespace_and_workdir_by_name() {
    for args in [&["serve"][..], &["serve", "--id", "fixture", "--namespace", "default"][..]] {
        let out = Command::new(env!("CARGO_BIN_EXE_grammard")).args(args).output().expect("grammard runs");
        assert_eq!(out.status.code(), Some(2), "a usage error: {out:?}");
        assert!(out.stdout.is_empty(), "nothing on stdout, the wire's");
    }
}

#[test]
fn an_unknown_command_is_a_usage_error() {
    for command in ["instance", "bridge", "cli"] {
        let out = Command::new(env!("CARGO_BIN_EXE_grammard")).arg(command).output().expect("grammard runs");
        assert_eq!(out.status.code(), Some(2), "{command}: {out:?}");
    }
}
