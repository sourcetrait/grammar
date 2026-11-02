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

/// A session lost before grammar-ssh's reply.
fn lost() -> TransportError {
    TransportError::SessionLost {
        reason: "grammar-ssh's stream ended before its reply".to_string(),
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
        assert!(reason.starts_with(&format!("grammar-ssh exited {status} before its reply")), "{reason}");
        assert!(reason.contains(lines[0]), "{reason}");
    }
}

#[test]
fn each_windows_shells_missing_command_is_command_not_found() {
    // As each shell answered on Windows Server 2025, PowerShell's lines in its colors.
    let cases: [(&str, &[&str]); 5] = [
        (
            "grammar-ssh",
            &[
                "\u{1b}[31;1mgrammar-ssh: \u{1b}[31;1mThe term 'grammar-ssh' is not recognized as a name of a cmdlet, function, script file, or executable program.\u{1b}[0m",
            ],
        ),
        (
            r"C:\opt\grammar\bin\grammar-ssh.exe",
            &[
                r"C:\opt\grammar\bin\grammar-ssh.exe: The term 'C:\opt\grammar\bin\grammar-ssh.exe' is not recognized as a name of a cmdlet, function, script file, or executable program.",
            ],
        ),
        (
            "grammar-ssh",
            &["'grammar-ssh' is not recognized as an internal or external command,", "operable program or batch file."],
        ),
        (r"C:\opt\grammar\bin\grammar-ssh.exe", &["The system cannot find the path specified."]),
        (r"C:\opt\grammar\bin\grammar-ssh.exe", &["   :            `-- Command `C:\\opt\\grammar\\bin\\grammar-ssh.exe` not found"]),
    ];
    for (command, lines) in cases {
        let refusal = witness(1, lines).explain(lost(), command);
        assert!(
            matches!(&refusal, TransportError::Unavailable { reason } if *reason == format!("command not found: {command}")),
            "{lines:?}: {refusal:?}"
        );
    }
}

#[test]
fn a_bare_commands_path_message_is_no_missing_command() {
    let refusal = witness(1, &["The system cannot find the path specified."]).explain(lost(), "grammar-ssh");
    let TransportError::Unavailable { reason } = &refusal else {
        panic!("{refusal:?}");
    };
    assert!(reason.starts_with("grammar-ssh exited 1 before its reply"), "{reason}");
}

#[test]
fn a_windows_hosts_grammar_ssh_line_goes_bare() {
    let endpoint = Endpoint {
        alias: "windows".to_string(),
        platform: Platform::Windows,
        host: "windows.example".to_string(),
        port: SSH_PORT,
        user: "worker".to_string(),
        key: "/keys/worker".to_string(),
        host_keys: Vec::new(),
        jump: None,
        command: r"C:\Users\worker\.sys\adhoc\opt\grammar\bin\grammar-ssh.exe".to_string(),
        id: "fixture".to_string(),
        namespace: "default".to_string(),
        workdir: Some(r"C:\Users\worker\work".to_string()),
    };
    let line = r"C:\Users\worker\.sys\adhoc\opt\grammar\bin\grammar-ssh.exe serve --id fixture --namespace default --workdir C:\Users\worker\work";
    assert_eq!(endpoint.grammar_ssh_line(), line);
    assert_eq!(endpoint.command_line(), line, "a Windows shell takes the line as it stands");
    let macos = Endpoint {
        platform: Platform::Macos,
        command: GRAMMAR_SSH_COMMAND.to_string(),
        workdir: None,
        ..endpoint
    };
    assert_eq!(
        macos.command_line(),
        "exec /bin/sh -c 'exec \"$SHELL\" -l -c \"exec grammar-ssh serve --id fixture --namespace default\"'",
        "macOS's login takes the Unix wrapper"
    );
}

#[test]
fn each_platform_holds_its_own_paths() {
    let windows = Platform::Windows;
    let linux = Platform::Linux;
    for good in [r"C:\Users\worker\.sys\adhoc\opt\grammar\bin\grammar-ssh.exe", r"d:\grammar-ssh.exe", "grammar-ssh"] {
        assert!(check_command(good, windows).is_ok(), "{good}");
    }
    for bad in [r"C:\Program Files\grammar\grammar-ssh.exe", "/opt/grammar/bin/grammar-ssh", r"\grammar-ssh.exe", r"C:grammar"] {
        assert!(check_command(bad, windows).is_err(), "{bad}");
    }
    assert!(check_path(r"C:\Users\worker\work", windows).is_ok());
    assert!(check_path(r"C:\Users\worker\my work", windows).is_err(), "a space parts the line");
    assert!(check_path("/srv/work", windows).is_err(), "a Unix path on Windows");
    assert!(check_command("/opt/grammar/bin/grammar-ssh", linux).is_ok());
    assert!(check_command(r"C:\opt\grammar-ssh.exe", linux).is_err(), "a drive path on Linux");
    assert!(check_path(r"C:\Users\worker\work", linux).is_err());
}

/// A host key's line, its key test-only.
const HOST_KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJdD7y3aLq454yWBdwLWbieU1ebz9/cu7/QEXn9OIeZJ example";

/// An endpoint of `alias` whose every field passes its check, reached directly.
fn whole(alias: &str) -> Endpoint {
    Endpoint {
        alias: alias.to_string(),
        platform: Platform::Linux,
        host: "build.example".to_string(),
        port: 2222,
        user: "worker".to_string(),
        key: "/keys/worker".to_string(),
        host_keys: vec![HOST_KEY.to_string()],
        jump: None,
        command: GRAMMAR_SSH_COMMAND.to_string(),
        id: "fixture".to_string(),
        namespace: "default".to_string(),
        workdir: None,
    }
}

#[test]
fn a_jump_names_its_hop_and_is_held_to_a_logins_checks() {
    let gate = Jump::of(&Endpoint {
        host: "gate.example".to_string(),
        port: 22,
        ..whole("gate")
    });
    let endpoint = Endpoint {
        jump: Some(gate.clone()),
        ..whole("lab")
    };
    assert!(endpoint.check().is_ok(), "{:?}", endpoint.check());
    assert_eq!(gate.place(), "the jump `gate` at gate.example:22");
    assert_eq!(endpoint.place(), "build.example:2222 through the jump `gate`");
    assert_eq!(whole("lab").place(), "build.example:2222", "a direct host names its address alone");
    let itself = Endpoint {
        jump: Some(Jump::of(&whole("lab"))),
        ..whole("lab")
    };
    assert!(itself.check().expect_err("a jump to itself").contains("jump names the entry itself"));
    let keyless = Endpoint {
        jump: Some(Jump {
            host_keys: Vec::new(),
            ..gate
        }),
        ..whole("lab")
    };
    let refusal = keyless.check().expect_err("a jump pins a key");
    assert!(refusal.starts_with("[[engine]] `gate`: known_hosts"), "the jump's own alias and field: {refusal}");
}

#[test]
fn a_platform_is_named_windows_linux_or_macos() {
    for name in Platform::NAMES {
        assert_eq!(Platform::parse(name).map(Platform::name), Ok(name));
    }
    assert!(Platform::parse("unix").is_err());
    assert!(Platform::parse("Windows").is_err());
    assert!(Platform::Linux.is_unix() && Platform::Macos.is_unix() && !Platform::Windows.is_unix());
}

#[test]
fn grammar_ssh_runs_through_the_accounts_login_shell() {
    let endpoint = Endpoint {
        alias: "build".to_string(),
        platform: Platform::Linux,
        host: "build.example".to_string(),
        port: SSH_PORT,
        user: "worker".to_string(),
        key: "/keys/worker".to_string(),
        host_keys: Vec::new(),
        jump: None,
        command: GRAMMAR_SSH_COMMAND.to_string(),
        id: "fixture".to_string(),
        namespace: "default".to_string(),
        workdir: None,
    };
    assert_eq!(endpoint.grammar_ssh_line(), "grammar-ssh serve --id fixture --namespace default");
    assert_eq!(
        endpoint.command_line(),
        "exec /bin/sh -c 'exec \"$SHELL\" -l -c \"exec grammar-ssh serve --id fixture --namespace default\"'"
    );
    let placed = Endpoint {
        workdir: Some("/srv/work".to_string()),
        ..endpoint
    };
    assert_eq!(placed.grammar_ssh_line(), "grammar-ssh serve --id fixture --namespace default --workdir /srv/work");
}
