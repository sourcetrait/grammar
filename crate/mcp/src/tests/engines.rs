use crate::*;
use crate::engines::{SshResolved, known_hosts_name, parse_engines, parse_known_keys, parse_ssh_g};

/// A pinned host key's OpenSSH line, its key test-only.
const HOST_KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJdD7y3aLq454yWBdwLWbieU1ebz9/cu7/QEXn9OIeZJ example";

/// An `[[engine]]` entry's fields, each valid, `alias` given.
fn engine_fields(alias: &str) -> Vec<String> {
    vec![
        format!("alias = \"{alias}\""),
        "host = \"build.example\"".to_string(),
        "user = \"worker\"".to_string(),
        "key = \"/keys/worker\"".to_string(),
        format!("host_keys = [\"{HOST_KEY}\"]"),
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

/// A remotes file with a comment and a channel, the parts an edit must keep.
const REMOTES: &str = "\
# the lab's remotes
[[channel]]
alias = \"peer\"
cert = \"/keys/peer.pem\"
";

#[test]
fn an_entry_opens_on_its_defaults_and_command_line() {
    let endpoint = one_entry(&engine_text(&engine_fields("build"))).endpoint().expect("a whole entry");
    assert_eq!((endpoint.port, endpoint.namespace.as_str()), (22, "default"), "the port and namespace default");
    assert_eq!(
        endpoint.command_line(),
        "/opt/grammar/bin/grammar-ssh serve --id fixture --namespace default --workdir /srv/work",
    );
    assert!(parse_engines("").expect("an empty file").is_empty(), "no engine unless the file names one");
    assert!(parse_engines(REMOTES).expect("channels alone").is_empty(), "a channel is no engine");
}

#[test]
fn an_incomplete_entry_is_kept_and_names_what_it_lacks() {
    let entry = one_entry("[[engine]]\nalias = \"build\"\nhost = \"build.example\"\n");
    assert_eq!(entry.missing(), ["user", "key", "host_keys", "command", "id", "workdir"]);
    let err = entry.endpoint().expect_err("an incomplete entry opens no session");
    for field in entry.missing() {
        assert!(err.contains(&format!("`{field}`")), "{field}: {err}");
    }
    assert!(one_entry(&engine_text(&engine_fields("build"))).missing().is_empty());
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
        ("alias = \"../up\"", "not a name"),
        ("alias = \"\"", "missing `alias`"),
        ("port = 0", "zero"),
        ("user = \"a user\"", "space"),
        ("key = \"keys/worker\"", "absolute"),
        ("host_keys = []", "empty"),
        ("host_keys = [\"not a key\"]", "OpenSSH public key"),
        ("command = \"bin/grammar-ssh\"", "absolute"),
        ("command = \"/opt/grammar $HOME/grammar-ssh\"", "login shell"),
        ("workdir = \"/srv/work;reboot\"", "login shell"),
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
fn a_set_makes_then_changes_an_entry_keeping_the_rest_of_the_file() {
    let change = EngineChange {
        host: Some("build.example".to_string()),
        user: Some("worker".to_string()),
        ..EngineChange::default()
    };
    let made = set_engine(REMOTES, "build", &change).expect("a new entry");
    assert!(made.starts_with(REMOTES), "the comment and the channel stand as they were: {made}");
    let entry = one_entry(&made);
    assert_eq!((entry.host.as_deref(), entry.user.as_deref(), entry.port), (Some("build.example"), Some("worker"), None));
    let port = EngineChange {
        port: Some(2222),
        ..EngineChange::default()
    };
    let changed = one_entry(&set_engine(&made, "build", &port).expect("a changed entry"));
    assert_eq!(changed.port, Some(2222), "the field given is written");
    assert_eq!(changed.host.as_deref(), Some("build.example"), "a field not given stays");
    assert_eq!(changed.user.as_deref(), Some("worker"), "a field not given stays");
}

#[test]
fn a_set_breaking_a_rule_is_refused() {
    let zero = EngineChange {
        port: Some(0),
        ..EngineChange::default()
    };
    let err = set_engine(REMOTES, "build", &zero).expect_err("port zero is refused");
    assert!(err.contains("zero"), "got {err}");
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
fn a_known_hosts_name_brackets_a_port_off_22() {
    assert_eq!(known_hosts_name("build.example", 22), "build.example");
    assert_eq!(known_hosts_name("build.example", 2222), "[build.example]:2222");
}
