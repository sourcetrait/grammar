use crate::*;
use crate::engines::{SshResolved, address_of, parse_engines, parse_known_keys, parse_ssh_g};

/// An entry's key line, its file absolute on this platform, as the MCP's local
/// path must be.
#[cfg(not(windows))]
const KEY_LINE: &str = "key = '/keys/worker'";
#[cfg(windows)]
const KEY_LINE: &str = r"key = 'C:\keys\worker'";

/// A pinned host key's OpenSSH line, its key test-only.
const HOST_KEY: &str ="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJdD7y3aLq454yWBdwLWbieU1ebz9/cu7/QEXn9OIeZJ example";

/// An `[[engine]]` entry's fields, each valid, `alias` given.
fn engine_fields(alias: &str) -> Vec<String> {
    vec![
        format!("alias = \"{alias}\""),
        "address = \"build.example\"".to_string(),
        "user = \"worker\"".to_string(),
        KEY_LINE.to_string(),
        format!("known_hosts = [\"{HOST_KEY}\"]"),
        "command = \"/opt/grammar/bin/grammar-ssh\"".to_string(),
        "id = \"fixture\"".to_string(),
        "workdir = \"/srv/work\"".to_string(),
    ]
}

/// An `[[engine]]` table of `fields`, each a line.
fn engine_text(fields: &[String]) -> String {
    format!("[[engine]]\n{}\n", fields.join("\n"))
}

/// The one entry `text` holds.
fn one_entry(text: &str) -> EngineEntry {
    let entries = parse_engines(text).expect("a valid file");
    let [entry] = entries.as_slice() else {
        panic!("one entry, not {entries:?}");
    };
    entry.clone()
}

/// The defaults of an MCP `agent` in namespace `lab`.
fn defaults() -> EngineDefaults {
    EngineDefaults {
        id: "agent".to_string(),
        namespace: "lab".to_string(),
        dir: PathBuf::from("/agent/.grammar"),
    }
}

/// A remotes file with a comment and a channel, the parts an edit must keep.
const REMOTES: &str = "\
# the lab's remotes
[[channel]]
alias = \"peer\"
cert = \"/keys/peer.pem\"
";

#[test]
fn an_entry_opens_on_its_fields_and_the_mcps_defaults() {
    let endpoint = one_entry(&engine_text(&engine_fields("build"))).endpoint(&defaults()).expect("a whole entry");
    assert_eq!((endpoint.host.as_str(), endpoint.port), ("build.example", 22), "port 22 by default");
    assert_eq!(endpoint.namespace, "lab", "the MCP's own namespace by default");
    assert_eq!(
        endpoint.worker_line(),
        "/opt/grammar/bin/grammar-ssh serve --id fixture --namespace lab --workdir /srv/work",
    );
    let least = vec![
        "alias = \"build\"".to_string(),
        "address = \"build.example:2222\"".to_string(),
        "user = \"worker\"".to_string(),
        KEY_LINE.to_string(),
        format!("known_hosts = [\"{HOST_KEY}\"]"),
    ];
    let endpoint = one_entry(&engine_text(&least)).endpoint(&defaults()).expect("the required fields alone");
    assert_eq!(endpoint.port, 2222);
    assert_eq!(endpoint.worker_line(), "grammar-ssh serve --id agent --namespace lab", "every default taken");
    assert!(parse_engines("").expect("an empty file").is_empty(), "no engine unless the file names one");
    assert!(parse_engines(REMOTES).expect("channels alone").is_empty(), "a channel is no engine");
}

#[test]
fn an_incomplete_entry_is_kept_and_names_what_it_lacks() {
    let entry = one_entry("[[engine]]\nalias = \"build\"\naddress = \"build.example\"\n");
    assert_eq!(entry.missing(), ["user", "key", "known_hosts"]);
    let err = entry.endpoint(&defaults()).expect_err("an incomplete entry opens no session");
    for field in entry.missing() {
        assert!(err.contains(&format!("`{field}`")), "{field}: {err}");
    }
    assert!(one_entry(&engine_text(&engine_fields("build"))).missing().is_empty());
}

#[test]
fn an_alias_is_names_joined_by_slashes() {
    assert_eq!(one_entry(&engine_text(&engine_fields("ai/lab"))).alias, "ai/lab");
    for alias in ["../up", "ai//lab", "/ai", "ai/", "ai/.hidden"] {
        let err = parse_engines(&engine_text(&engine_fields(alias))).expect_err(alias);
        assert!(err.contains("not a name"), "{alias}: {err}");
    }
}

#[test]
fn an_entry_breaking_a_rule_is_refused() {
    let with = |line: &str| -> String {
        let field = line.split(' ').next().unwrap_or_default();
        let mut fields: Vec<String> =
            engine_fields("build").into_iter().filter(|known| !known.starts_with(&format!("{field} "))).collect();
        fields.push(line.to_string());
        engine_text(&fields)
    };
    for (line, says) in [
        ("alias = \"local\"", "MCP's own engine"),
        ("alias = \"\"", "missing `alias`"),
        ("address = \"build.example:0\"", "isn't a port"),
        ("address = \"build.example:ssh\"", "isn't a port"),
        ("address = \"::1\"", "brackets"),
        ("address = \"[::1\"", "never closes"),
        ("address = \"a host\"", "space"),
        ("user = \"a user\"", "space"),
        ("key = \"keys/worker\"", "absolute"),
        ("known_hosts = []", "empty"),
        ("known_hosts = [\"not a key\"]", "OpenSSH public key"),
        ("known_hosts = \"\"", "empty"),
        ("command = \"bin/grammar-ssh\"", "neither"),
        ("command = \"/opt/grammar $HOME/grammar-ssh\"", "login shell"),
        ("workdir = \"/srv/work;reboot\"", "login shell"),
        ("workdir = \"srv/work\"", "absolute"),
        ("id = \"-x\"", "instance name"),
    ] {
        let err = parse_engines(&with(line)).expect_err(line);
        assert!(err.contains(says), "{line}: {err}");
    }
    let twice = format!("{}{}", engine_text(&engine_fields("build")), engine_text(&engine_fields("build")));
    assert!(parse_engines(&twice).expect_err("one alias twice").contains("duplicate"));
    let stray = format!("{}stray = 1\n", engine_text(&engine_fields("build")));
    assert!(parse_engines(&stray).is_err(), "an unknown field is refused");
}

#[test]
fn a_retired_field_is_refused_naming_what_took_its_place() {
    let keys = format!("host_keys = [\"{HOST_KEY}\"]");
    for (line, says) in [
        ("host = \"build.example\"", "`host` is retired; `address` takes its place"),
        ("port = 22", "`port` is retired; `address` takes its place"),
        (keys.as_str(), "`host_keys` is retired; `known_hosts` takes its place"),
    ] {
        let mut fields = engine_fields("build");
        fields.push(line.to_string());
        let err = parse_engines(&engine_text(&fields)).expect_err(line);
        assert!(err.contains(says), "{line}: {err}");
    }
}

#[test]
fn known_hosts_are_a_list_of_keys_or_a_file() {
    let file = one_entry(&engine_text(
        &engine_fields("build")
            .into_iter()
            .map(|field| if field.starts_with("known_hosts ") { "known_hosts = \"remote/build/known_hosts\"".to_string() } else { field })
            .collect::<Vec<_>>(),
    ));
    assert_eq!(file.known_hosts, Some(KnownHosts::File("remote/build/known_hosts".to_string())));
    let keys = one_entry(&engine_text(&engine_fields("build")));
    assert_eq!(keys.known_hosts, Some(KnownHosts::Keys(vec![HOST_KEY.to_string()])));
}

#[test]
fn a_set_makes_then_changes_an_entry_keeping_the_rest_of_the_file() {
    let change = EngineChange {
        address: Some("build.example".to_string()),
        user: Some("worker".to_string()),
        ..EngineChange::default()
    };
    let made = set_engine(REMOTES, "build", &change).expect("a new entry");
    assert!(made.starts_with(REMOTES), "the comment and the channel stand as they were: {made}");
    let entry = one_entry(&made);
    assert_eq!((entry.address.as_deref(), entry.user.as_deref()), (Some("build.example"), Some("worker")));
    let file = EngineChange {
        address: Some("build.example:2222".to_string()),
        known_hosts: Some(KnownHosts::File("remote/build/known_hosts".to_string())),
        ..EngineChange::default()
    };
    let changed = set_engine(&made, "build", &file).expect("a changed entry");
    assert!(changed.contains("known_hosts = \"remote/build/known_hosts\""), "a file is a string: {changed}");
    let changed = one_entry(&changed);
    assert_eq!(changed.address.as_deref(), Some("build.example:2222"), "the field given is written");
    assert_eq!(changed.user.as_deref(), Some("worker"), "a field not given stays");
    let keys = EngineChange {
        known_hosts: Some(KnownHosts::Keys(vec![HOST_KEY.to_string()])),
        ..EngineChange::default()
    };
    let keyed = one_entry(&set_engine(&made, "build", &keys).expect("keys inline"));
    assert_eq!(keyed.known_hosts, keys.known_hosts, "the keys replace the file");
}

#[test]
fn a_set_moves_an_older_entry_to_the_new_fields() {
    let older = format!(
        "{REMOTES}[[engine]]\nalias = \"build\"\nhost = \"build.example\"\nport = 2222\nhost_keys = [\"{HOST_KEY}\"]\n"
    );
    assert!(parse_engines(&older).is_err(), "an older entry is refused as it stands");
    let half = EngineChange {
        address: Some("build.example:2222".to_string()),
        ..EngineChange::default()
    };
    let err = set_engine(&older, "build", &half).expect_err("host_keys still stands");
    assert!(err.contains("`host_keys` is retired"), "{err}");
    let whole = EngineChange {
        known_hosts: Some(KnownHosts::Keys(vec![HOST_KEY.to_string()])),
        ..half
    };
    let moved = set_engine(&older, "build", &whole).expect("the older fields dropped");
    let entry = one_entry(&moved);
    assert_eq!(entry.address.as_deref(), Some("build.example:2222"));
    for retired in ["host =", "port =", "host_keys ="] {
        assert!(!moved.lines().any(|line| line.starts_with(retired)), "{retired}: {moved}");
    }
}

#[test]
fn a_set_breaking_a_rule_is_refused() {
    let zero = EngineChange {
        address: Some("build.example:0".to_string()),
        ..EngineChange::default()
    };
    let err = set_engine(REMOTES, "build", &zero).expect_err("port zero is refused");
    assert!(err.contains("isn't a port"), "got {err}");
    assert!(set_engine(REMOTES, "local", &EngineChange::default()).is_err(), "`local` names the MCP's engine");
}

#[test]
fn an_unset_removes_its_entry_alone() {
    let two = format!("{REMOTES}{}{}", engine_text(&engine_fields("build")), engine_text(&engine_fields("spare")));
    let one = unset_engine(&two, "build").expect("the entry is removed");
    assert!(one.starts_with(REMOTES), "the comment and the channel stand as they were: {one}");
    assert_eq!(one_entry(&one).alias, "spare", "the other entry stays");
    assert!(unset_engine(&one, "build").expect_err("no entry twice").contains("no [[engine]]"));
    assert!(unset_engine(REMOTES, "build").is_err(), "a file with no engines has none to remove");
}

#[test]
fn ssh_g_output_gives_the_fields_an_entry_takes() {
    let output = "\
host build
hostname build.example
port 2222
user worker
identityfile ~/.ssh/id_ed25519
identityfile ~/.ssh/id_rsa
userknownhostsfile ~/.ssh/known_hosts ~/.ssh/known_hosts2
globalknownhostsfile /etc/ssh/ssh_known_hosts
hostkeyalias none
";
    assert_eq!(
        parse_ssh_g(output),
        SshResolved {
            hostname: Some("build.example".to_string()),
            port: Some(2222),
            user: Some("worker".to_string()),
            identity_files: vec!["~/.ssh/id_ed25519".to_string(), "~/.ssh/id_rsa".to_string()],
            known_hosts_files: vec![
                "~/.ssh/known_hosts".to_string(),
                "~/.ssh/known_hosts2".to_string(),
                "/etc/ssh/ssh_known_hosts".to_string(),
            ],
            host_key_alias: None,
        },
    );
    assert_eq!(parse_ssh_g("hostkeyalias build-key\n").host_key_alias.as_deref(), Some("build-key"));
}

#[test]
fn known_keys_are_taken_once_each_their_markers_skipped() {
    let output = format!(
        "\
# Host build.example found: line 3
build.example {HOST_KEY}
@revoked build.example ssh-rsa AAAArevoked
|1|c2FsdA==|aGFzaA== ecdsa-sha2-nistp256 AAAAecdsa
build.example {HOST_KEY}
"
    );
    let pinned = HOST_KEY.rsplit_once(' ').map(|(key, _comment)| key).unwrap_or(HOST_KEY);
    assert_eq!(parse_known_keys(&output), [pinned.to_string(), "ecdsa-sha2-nistp256 AAAAecdsa".to_string()]);
}

#[test]
fn an_address_and_a_known_hosts_name_bracket_what_they_must() {
    assert_eq!(known_hosts_name("build.example", 22), "build.example");
    assert_eq!(known_hosts_name("build.example", 2222), "[build.example]:2222");
    assert_eq!(address_of("build.example", 22), "build.example");
    assert_eq!(address_of("build.example", 2222), "build.example:2222");
    assert_eq!(address_of("::1", 22), "[::1]");
    assert_eq!(address_of("::1", 2222), "[::1]:2222");
    for (address, parsed) in [
        ("build.example", ("build.example", 22)),
        ("build.example:2222", ("build.example", 2222)),
        ("[::1]", ("::1", 22)),
        ("[::1]:2222", ("::1", 2222)),
    ] {
        assert_eq!(parse_address(address).expect(address), (parsed.0.to_string(), parsed.1));
    }
}
