use crate::*;
use crate::config::ConfigToml;

/// A runtime config from file text alone, its argument-owned values placeholders.
fn from_text(text: &str) -> Result<Config, String> {
    // The placeholders stand in because the file surface is what these tests lock.
    let toml: ConfigToml = toml::from_str(text).map_err(|e| e.to_string())?;
    Config::from_toml(
        toml,
        "id".to_string(),
        "ns".to_string(),
        PathBuf::from("/w"),
        DenySet::default(),
        false,
        PathBuf::from("/w/.grammar/remotes.toml"),
    )
}

/// An `[identity]` table naming a key pair under `/keys`.
const IDENTITY: &str = "\
[identity]
cert = \"/keys/self.pem\"
key = \"/keys/self.key.pem\"
";

#[test]
fn an_empty_file_yields_the_embedded_defaults() {
    let config = from_text("").expect("empty file is valid");
    assert_eq!(
        config.channel.port, None,
        "an unpinned port must stay None, not become a 0 sentinel",
    );
    assert!(config.identity.is_none(), "the embedded base names no identity");
}

#[test]
fn a_privileged_or_zero_port_is_rejected() {
    // 0 is the syscall's "assign me one" convention, never a configurable value; below
    // 1024 needs root, which the host never has, so it could only fail at bind.
    for port in [0, 80, 443, 1023] {
        let err = from_text(&format!("[channel]\nport = {port}\n"))
            .expect_err(&format!("port {port} must be rejected"));
        assert!(err.contains("1024-65535"), "port {port}: got {err}");
    }
}

#[test]
fn the_lowest_unprivileged_port_is_accepted() {
    let config = from_text("[channel]\nport = 1024\n").expect("1024 is valid");
    assert_eq!(config.channel.port, Some(1024));
}

#[test]
fn the_namespace_cannot_be_set_from_a_file() {
    // The whole point of deny_unknown_fields here: a file must not be able to move the
    // namespace out from under the .mcp.json entry that launched the server.
    for text in [
        "id = \"other\"\n",
        "namespace = \"other\"\n",
        "work_dir = \"/elsewhere\"\n",
        "deny = [\"run\"]\n",
        // `--test` joins the argument-owned set for the same reason: it selects
        // the namespace, so a file-settable one could move the namespace out from
        // under the .mcp.json entry that launched the server.
        "test = true\n",
    ] {
        assert!(
            from_text(text).is_err(),
            "{text:?} must be rejected as an unknown field",
        );
    }
}

#[test]
fn the_test_flag_is_carried_on_the_runtime_config() {
    let toml: ConfigToml = toml::from_str("").expect("empty file is valid");
    let config = Config::from_toml(
        toml,
        "id".to_string(),
        TEST_NAMESPACE.to_string(),
        PathBuf::from("/w"),
        DenySet::default(),
        true,
        PathBuf::from("/w/.grammar/remotes.toml"),
    )
    .expect("valid");
    assert!(config.test, "the watchdog half reads this at every tick");
    assert!(
        !from_text("").expect("valid").test,
        "and an ordinary host is not a test host",
    );
}

#[test]
fn a_retired_channel_key_is_rejected() {
    // bind / cert_name / verify_timeout_secs / cert_dir were removed; a stale file must
    // fail loudly rather than be silently ignored.
    for text in [
        "[channel]\nbind = \"0.0.0.0\"\n",
        "[channel]\ncert_name = \"custom\"\n",
        "[channel]\nverify_timeout_secs = 42\n",
        "[channel]\ncert_dir = \"/somewhere/certs\"\n",
    ] {
        assert!(from_text(text).is_err(), "{text:?} must be rejected");
    }
}

#[test]
fn an_identity_names_both_files_expanded() {
    let config =
        from_text("[identity]\ncert = \"~/k/self.pem\"\nkey = \"~/k/self.key.pem\"\n")
            .expect("valid");
    let identity = config.identity.expect("an identity");
    assert!(
        identity.cert.is_absolute() && identity.key.is_absolute(),
        "a `~` path must be expanded at load; got {} and {}",
        identity.cert.display(),
        identity.key.display(),
    );
    assert!(identity.cert.ends_with("k/self.pem"));
    assert!(identity.key.ends_with("k/self.key.pem"));
}

#[test]
fn an_identity_missing_a_file_is_rejected() {
    for (text, field) in [
        ("[identity]\ncert = \"/k/self.pem\"\n", "key"),
        ("[identity]\nkey = \"/k/self.key.pem\"\n", "cert"),
    ] {
        let err = from_text(text).expect_err("an identity names both files");
        assert!(err.contains(&format!("`{field}`")), "{text:?}: got {err}");
    }
}

#[test]
fn a_field_by_its_former_name_is_rejected() {
    // `cert_file` and `key_file` became `cert` and `key`; a file still naming the
    // old ones must fail loudly.
    for text in [
        "[identity]\ncert_file = \"/k/self.pem\"\nkey = \"/k/self.key.pem\"\n",
        "[identity]\ncert = \"/k/self.pem\"\nkey_file = \"/k/self.key.pem\"\n",
    ] {
        assert!(from_text(text).is_err(), "{text:?} must be rejected");
    }
}

#[test]
fn a_remote_in_the_config_file_is_rejected() {
    // The remotes live in `remotes.toml` beside the config; one left here is stale.
    let text = format!("{IDENTITY}\n[[remote]]\nalias = \"bob\"\ncert = \"/keys/bob.pem\"\n");
    let err = from_text(&text).expect_err("a [[remote]] in the config file must fail");
    assert!(err.contains("remote"), "got {err}");
}

#[test]
fn an_engine_in_the_config_file_is_rejected() {
    // The engines moved to `remotes.toml` beside the config; one left here is stale.
    let err = from_text("[[engine]]\nalias = \"build\"\n").expect_err("an [[engine]] in the config file must fail");
    assert!(err.contains("engine"), "got {err}");
}

#[test]
fn an_unset_variable_fails_the_load() {
    // Expansion happens at the boundary, so an unresolvable path is a config error
    // rather than something that surfaces much later at first use. Only a home, a
    // UENV one or a standard one, falls back; any other unset variable is
    // unresolvable.
    let err = from_text(
        "[identity]\ncert = \"$GRAMMAR_TEST_UNSET_VAR/self.pem\"\nkey = \"/k\"\n",
    )
    .expect_err("an unset variable outside the XDG homes must fail the load");
    assert!(err.contains("GRAMMAR_TEST_UNSET_VAR"), "got {err}");
}

#[test]
fn the_xdg_homes_carry_spec_defaults() {
    // The expansion context falls back to these when a variable is unset or empty;
    // every other variable has none and stays a load error.
    let home = PathBuf::from(std::env::var("HOME").expect("HOME set"));
    for (var, home_relative) in [
        ("XDG_CACHE_HOME", ".cache"),
        ("XDG_CONFIG_HOME", ".config"),
        ("XDG_DATA_HOME", ".local/share"),
        ("XDG_STATE_HOME", ".local/state"),
    ] {
        assert_eq!(
            crate::config::spec_default(var),
            Some(home.join(home_relative)),
            "{var} must default under $HOME as the XDG spec has it",
        );
    }
    for var in ["XDG_RUNTIME_DIR", "GRAMMAR_TEST_UNSET_VAR"] {
        assert_eq!(
            crate::config::spec_default(var),
            None,
            "{var} has no default and stays a load error",
        );
    }
}

#[test]
fn a_uenv_home_stands_in_front_of_its_standard_variable() {
    for (uenv, standard) in [
        ("UENV_USR_CACHE", "XDG_CACHE_HOME"),
        ("UENV_USR_CONFIG", "XDG_CONFIG_HOME"),
        ("UENV_USR_DATA", "XDG_DATA_HOME"),
        ("UENV_USR_STATE", "XDG_STATE_HOME"),
        ("UENV_USR_TMP", "TMPDIR"),
        ("UENV_USR_TRANSIENT", "XDG_RUNTIME_DIR"),
    ] {
        assert_eq!(
            crate::home::uenv_standard(uenv),
            Some(standard),
            "{uenv} stands in front of {standard}",
        );
        assert_eq!(crate::home::uenv_partner(standard), Some(uenv), "and {standard} behind {uenv}");
    }
    assert_eq!(
        crate::home::uenv_standard("UENV_USR_SECRET"),
        None,
        "a UENV tree with no standard variable stands alone",
    );
    assert_eq!(
        crate::config::spec_default("TMPDIR"),
        Some(PathBuf::from("/tmp")),
        "TMPDIR defaults to /tmp",
    );
}

#[test]
fn the_agent_file_is_read_from_its_directory() {
    let dir = tempfile::tempdir().expect("a directory");
    let none = Config::read_agent_toml(dir.path()).expect("an absent file is none");
    assert!(none.identity.is_none());
    let grammar = dir.path().join(".grammar");
    fs::create_dir_all(&grammar).expect("its .grammar");
    fs::write(grammar.join("config.toml"), "[channel]\nport = 47800\n").expect("its file");
    let file = Config::read_agent_toml(dir.path()).expect("the file is read");
    assert_eq!(file.channel.and_then(|channel| channel.port), Some(47800));
}
