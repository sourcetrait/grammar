use crate::*;
use crate::ssh::Witness;

/// A witness of an exit of `status` with these stderr lines.
fn witness(status: u32, lines: &[&str]) -> Witness {
    let mut witness = Witness {
        exit: Some(format!("exited {status}")),
        status: Some(status),
        ..Witness::default()
    };
    for line in lines {
        witness.keep(line.to_string());
    }
    witness
}

/// A session lost before the worker's reply.
fn lost() -> TransportError {
    TransportError::SessionLost {
        reason: "the worker's stream ended before its reply".to_string(),
    }
}

#[test]
fn a_missing_absolute_command_is_command_not_found_by_each_sh() {
    let command = "/opt/grammar/bin/grammar-ssh";
    let cases: [(u32, &[&str]); 3] = [
        (127, &["/bin/sh: line 1: /opt/grammar/bin/grammar-ssh: No such file or directory"]),
        (
            126,
            &[
                "/bin/sh: /opt/grammar/bin/grammar-ssh: No such file or directory",
                "/bin/sh: line 0: exec: /opt/grammar/bin/grammar-ssh: cannot execute: No such file or directory",
            ],
        ),
        (127, &["zsh:1: no such file or directory: /opt/grammar/bin/grammar-ssh"]),
    ];
    for (status, lines) in cases {
        let refusal = witness(status, lines).explain(lost(), command);
        assert!(
            matches!(&refusal, TransportError::Unavailable { reason } if *reason == format!("command not found: {command}")),
            "{lines:?}: {refusal:?}"
        );
    }
}

#[test]
fn each_login_shells_missing_command_is_command_not_found() {
    let cases: [(u32, &[&str]); 6] = [
        (127, &["/bin/bash: line 1: exec: grammar-ssh: not found"]),
        (127, &["zsh:1: command not found: grammar-ssh"]),
        (127, &["/bin/sh: 1: exec: grammar-ssh: not found"]),
        (127, &["fish: Unknown command: grammar-ssh"]),
        (
            1,
            &[
                "Error: nu::shell::external_command",
                "",
                "  x External command failed",
                "   :   `-- Command `grammar-ssh` not found",
                "   `----",
            ],
        ),
        (1, &["  help: `grammar-ssh` is neither a Nushell built-in or a known external", "        command"]),
    ];
    for (status, lines) in cases {
        let refusal = witness(status, lines).explain(lost(), "grammar-ssh");
        assert!(
            matches!(&refusal, TransportError::Unavailable { reason } if reason == "command not found: grammar-ssh"),
            "{lines:?}: {refusal:?}"
        );
    }
}

#[test]
fn another_failure_keeps_its_exit_and_its_stderr() {
    // 127 naming another command, 126 for a command found but not runnable, and nu's 1 otherwise.
    let cases: [(u32, &[&str]); 3] = [
        (127, &["/bin/sh: 1: exec: /bin/zzz: not found"]),
        (126, &["/bin/bash: line 1: /opt/grammar/bin/grammar-ssh: Permission denied"]),
        (1, &["Error: nu::shell::io_error"]),
    ];
    for (status, lines) in cases {
        let refusal = witness(status, lines).explain(lost(), "grammar-ssh");
        let TransportError::Unavailable { reason } = &refusal else {
            panic!("{refusal:?}");
        };
        assert!(reason.starts_with(&format!("the worker exited {status} before its reply")), "{reason}");
        assert!(reason.contains(lines[0]), "{reason}");
    }
}

#[test]
fn the_worker_runs_through_the_accounts_login_shell() {
    let endpoint = Endpoint {
        alias: "build".to_string(),
        host: "build.example".to_string(),
        port: SSH_PORT,
        user: "worker".to_string(),
        key: "/keys/worker".to_string(),
        host_keys: Vec::new(),
        command: WORKER_COMMAND.to_string(),
        id: "fixture".to_string(),
        namespace: "default".to_string(),
        workdir: None,
    };
    assert_eq!(endpoint.worker_line(), "grammar-ssh serve --id fixture --namespace default");
    assert_eq!(
        endpoint.command_line(),
        "exec /bin/sh -c 'exec \"$SHELL\" -l -c \"exec grammar-ssh serve --id fixture --namespace default\"'"
    );
    let placed = Endpoint {
        workdir: Some("/srv/work".to_string()),
        ..endpoint
    };
    assert_eq!(placed.worker_line(), "grammar-ssh serve --id fixture --namespace default --workdir /srv/work");
}
