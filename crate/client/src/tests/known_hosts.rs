use crate::*;

/// Keys made by `ssh-keygen` for these tests alone.
const KEY_A: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAikfipHrS+1TmZsZzXZyGqQW1LW7wCDsnhQYO3+rcjy";
const KEY_B: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFAs0hRxbQym61QdUfCoyqL3FSQ4OIszZApyG0esutaL";
const KEY_C: &str = "ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBBwotPz7mC4Jyweu7t7Ta3n4LoxdXaePufHnJXBHBXPbVlp22kYgjQtyYJko2tVaU7rI5qL9sNQGG5QWYvk3h+I=";

/// `build.example` and `[build.example]:2222` hashed by `ssh-keygen -H`.
const HASHED_22: &str = "|1|0DhyX3EBa1bjd73Ityz7KvibVfY=|AReNXZOnPdFJWbm3crYSr6zG92A=";
const HASHED_2222: &str = "|1|Rf6w8CtHq2C0B7Neoca4cA6MOuA=|8mZ6lRoF+dcOW13cWBOy9amGUxQ=";

/// The keys a file's `text` holds for `host` at `port`.
fn found(text: &str, host: &str, port: u16) -> Vec<String> {
    known_host_keys(text, host, port).expect("a known_hosts file")
}

#[test]
fn a_host_matches_by_name_and_off_22_by_bracketed_name() {
    let text = format!("build.example {KEY_A}\n[build.example]:2222 {KEY_B}\nother.example {KEY_C}\n");
    assert_eq!(found(&text, "build.example", 22), [KEY_A]);
    assert_eq!(found(&text, "build.example", 2222), [KEY_B]);
    assert_eq!(found(&text, "BUILD.example", 22), [KEY_A], "names compare without case");
    assert!(found(&text, "build.example", 2200).is_empty(), "no line names that port");
    assert!(found(&text, "missing.example", 22).is_empty());
}

#[test]
fn a_line_names_hosts_by_list_glob_and_negation() {
    let text = format!(
        "alpha.example,build.example {KEY_A}\n*.lab.example,!gate.lab.example {KEY_B}\nnode?.example {KEY_C}\n"
    );
    assert_eq!(found(&text, "build.example", 22), [KEY_A], "one name of a list");
    assert_eq!(found(&text, "w1.lab.example", 22), [KEY_B], "a glob");
    assert!(found(&text, "gate.lab.example", 22).is_empty(), "a negation wins over a glob");
    assert_eq!(found(&text, "node7.example", 22), [KEY_C], "`?` takes one character");
    assert!(found(&text, "node17.example", 22).is_empty(), "`?` takes no more than one");
    let bracketed = format!("[*.lab.example]:2222 {KEY_A}\n");
    assert_eq!(found(&bracketed, "w1.lab.example", 2222), [KEY_A], "a glob inside brackets");
    assert!(found(&bracketed, "w1.lab.example", 22).is_empty());
}

#[test]
fn a_hashed_name_matches_its_host_alone() {
    let text = format!("{HASHED_22} {KEY_A}\n{HASHED_2222} {KEY_B}\n");
    assert_eq!(found(&text, "build.example", 22), [KEY_A]);
    assert_eq!(found(&text, "build.example", 2222), [KEY_B]);
    assert!(found(&text, "other.example", 22).is_empty());
}

#[test]
fn a_revoked_key_is_never_pinned_and_a_ca_line_is_skipped() {
    let text = format!(
        "build.example {KEY_A}\nbuild.example {KEY_B}\n@revoked * {KEY_B}\n@cert-authority build.example {KEY_C}\n"
    );
    assert_eq!(found(&text, "build.example", 22), [KEY_A]);
}

#[test]
fn comments_blank_lines_tabs_and_repeats_are_read_as_ssh_reads_them() {
    let text = format!("# the lab's hosts\n\n  build.example\t{KEY_A}  a comment\nbuild.example {KEY_A}\n");
    assert_eq!(found(&text, "build.example", 22), [KEY_A], "one key once");
}

#[test]
fn a_malformed_line_is_refused_by_its_number() {
    let text = format!("build.example {KEY_A}\nbuild.example ssh-ed25519\n");
    let err = known_host_keys(&text, "build.example", 22).expect_err("refused");
    assert!(err.starts_with("line 2 "), "{err}");
}
