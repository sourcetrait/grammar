use crate::*;
use crate::remotes::parse_remotes;

/// A whole entry for `bob`, each field named.
const BOB: &str = "\
[[channel]]
alias = \"bob\"
cert = \"~/keys/bob.pem\"
address = \"127.0.0.1:9001\"
listen = \"127.0.0.1\"
";

/// An address the test names, parsed.
fn addr(text: &str) -> SocketAddr {
    text.parse().expect("an address")
}

/// The aliases of a file's entries, in its order.
fn aliases(text: &str) -> Vec<String> {
    parse_remotes(text)
        .expect("valid")
        .into_iter()
        .map(|entry| entry.alias)
        .collect()
}

#[test]
fn an_entry_keeps_its_fields_as_written() {
    let entries = parse_remotes(BOB).expect("valid");
    let [bob] = entries.as_slice() else {
        panic!("one entry: {entries:?}");
    };
    assert_eq!(bob.alias, "bob");
    assert_eq!(bob.cert, "~/keys/bob.pem", "the path stays as written");
    assert_eq!(bob.address.as_deref(), Some("127.0.0.1:9001"));
    assert_eq!(bob.listen.as_deref(), Some("127.0.0.1"));
    assert_eq!(bob.dial(), Some(addr("127.0.0.1:9001")));
    assert_eq!(bob.bind(), addr("127.0.0.1:0"), "a bare ip binds a random port");
}

#[test]
fn a_listen_defaults_to_loopback_on_a_random_port() {
    let entries = parse_remotes("[[channel]]\nalias = \"bob\"\ncert = \"/k/bob.pem\"\n")
        .expect("valid");
    assert_eq!(entries[0].bind(), addr("127.0.0.1:0"));
    assert_eq!(entries[0].dial(), None, "no address, no default dial");
    assert_eq!(
        bind_address("0.0.0.0:7000").expect("valid"),
        addr("0.0.0.0:7000"),
        "every interface only when named",
    );
}

#[test]
fn a_malformed_entry_is_refused() {
    for (text, needle) in [
        ("[[channel]]\ncert = \"/k\"\n", "`alias`"),
        ("[[channel]]\nalias = \"bob\"\n", "`cert`"),
        ("[[channel]]\nalias = \"bob\"\ncert = \"/k\"\naddress = \"nowhere\"\n", "is not ip:port"),
        ("[[channel]]\nalias = \"bob\"\ncert = \"/k\"\naddress = \"127.0.0.1\"\n", "is not ip:port"),
        ("[[channel]]\nalias = \"bob\"\ncert = \"/k\"\nlisten = \"nowhere\"\n", "neither ip"),
        ("[[channel]]\nalias = \"bob\"\ncert = \"/k\"\npublic_key_file = \"/k\"\n", "public_key_file"),
        ("[[channel]]\nalias = \"bob\"\ncert = \"/k\"\nself_public_key_file = \"/k\"\n", "self_public"),
        ("[identity]\ncert = \"/k\"\n", "identity"),
    ] {
        let err = parse_remotes(text).expect_err(text);
        assert!(err.contains(needle), "{text:?}: got {err}");
    }
    let twice = format!("{BOB}{BOB}");
    let err = parse_remotes(&twice).expect_err("an alias names one peer");
    assert!(err.contains("duplicate"), "got {err}");
}

#[test]
fn a_set_makes_a_new_entry_and_keeps_the_file_around_it() {
    let text = "# peers of this agent\n\n[[channel]]\n# a peer\nalias = \"peer\"\n\
                cert = \"/k/peer.pem\"\n";
    let change = RemoteChange {
        cert: Some("/k/bob.pem".to_string()),
        address: Some("127.0.0.1:9001".to_string()),
        listen: None,
    };
    let edited = set_entry(text, "bob", &change).expect("a new entry");
    assert!(edited.starts_with("# peers of this agent\n"), "{edited}");
    assert!(edited.contains("# a peer\nalias = \"peer\""), "{edited}");
    assert_eq!(aliases(&edited), ["peer", "bob"]);
    let bob = &parse_remotes(&edited).expect("valid")[1];
    assert_eq!(bob.address.as_deref(), Some("127.0.0.1:9001"));
    assert_eq!(bob.cert, "/k/bob.pem");
}

#[test]
fn a_set_changes_only_the_fields_it_gives() {
    let change = RemoteChange {
        listen: Some("127.0.0.1:9100".to_string()),
        ..RemoteChange::default()
    };
    let edited = set_entry(BOB, "bob", &change).expect("a change");
    let bob = &parse_remotes(&edited).expect("valid")[0];
    assert_eq!(bob.listen.as_deref(), Some("127.0.0.1:9100"));
    assert_eq!(bob.address.as_deref(), Some("127.0.0.1:9001"), "the rest as it was");
    assert_eq!(bob.cert, "~/keys/bob.pem");
}

#[test]
fn a_set_that_would_break_the_file_is_refused() {
    let change = RemoteChange {
        address: Some("127.0.0.1:9".to_string()),
        ..RemoteChange::default()
    };
    let err = set_entry("", "bob", &change).expect_err("a new entry needs its cert");
    assert!(err.contains("`cert`"), "got {err}");
}

#[test]
fn the_engines_entries_stand_beside_the_channels() {
    let text = format!("{BOB}\n[[engine]]\nalias = \"lab\"\nhost = \"lab.example\"\n");
    assert_eq!(aliases(&text), ["bob"], "an engine is no channel");
    let change = RemoteChange {
        listen: Some("127.0.0.1:9100".to_string()),
        ..RemoteChange::default()
    };
    let edited = set_entry(&text, "bob", &change).expect("a channel's change");
    assert!(edited.contains("[[engine]]\nalias = \"lab\"\nhost = \"lab.example\""), "{edited}");
    let unset = unset_entry(&edited, "bob").expect("bob removed");
    assert!(unset.contains("[[engine]]"), "{unset}");
}

#[test]
fn an_unset_removes_its_entry_alone() {
    let text = format!("{BOB}\n# a peer\n[[channel]]\nalias = \"peer\"\ncert = \"/k/peer.pem\"\n");
    let edited = unset_entry(&text, "bob").expect("bob removed");
    assert!(edited.contains("# a peer"), "{edited}");
    assert_eq!(aliases(&edited), ["peer"]);
    let err = unset_entry(&edited, "bob").expect_err("bob is gone");
    assert!(err.contains("`bob`"), "got {err}");
}
