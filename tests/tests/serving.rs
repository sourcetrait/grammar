//! The bare entry serves MCP on its own engine: it lists the four tools, holds
//! the deny set and the contract, survives a nested command, and serves the
//! Channel's commands itself; the remote channels' own cases are
//! `remote_channel`'s.

use std::time::Duration;

use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// A definition the host's engine answers.
const DEFINITION: &str =
    "def execute [data: record<x: int>]: nothing -> record<out: int> { { out: ($data.x + 1) } }";

#[tested]
fn serving_lists_the_four_tools() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());
    let mut names = host.tool_names();
    names.sort();
    assert_eq!(names, ["call", "execute", "interface", "rerun"]);
}

#[tested]
fn a_denied_tool_is_withheld() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--deny", "execute"]);
    let mut names = host.tool_names();
    names.sort();
    assert_eq!(names, ["call", "interface", "rerun"]);
}

#[tested]
fn a_host_the_server_does_not_know_is_refused_first() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());
    let answer = host.call(
        "execute",
        serde_json::json!({"host": "elsewhere", "definition": DEFINITION, "data": "{x: 5}"}),
    );
    assert_eq!(envelope_error_kind(&answer).as_deref(), Some("host::unknown"), "got {answer}");
}

#[tested]
fn a_command_off_its_contract_is_refused() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());
    let answer = host.interface("interface info | interface purview list");
    assert_eq!(
        envelope_error_kind(&answer).as_deref(),
        Some("interface::invalid"),
        "got {answer}",
    );
}

/// A deeply nested command meets nushell's parse on a worker sized for it: the
/// contract refuses it, and the host still serves.
#[tested]
fn a_nested_command_leaves_the_host_alive() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());
    let depth = 200;
    let command = format!("interface help {}'x'{}", "(".repeat(depth), ")".repeat(depth));
    let id = host.request_interface(&command);
    let answer = host.try_read_id(id).unwrap_or_else(|| {
        let end = host
            .wait_for_exit(Duration::from_secs(5))
            .map(|status| describe_exit(&status))
            .unwrap_or_else(|| "kept running".to_string());
        panic!("no answer to a command nested {depth} deep: the host {end}");
    });
    assert_eq!(
        envelope_error_kind(&answer).as_deref(),
        Some("interface::invalid"),
        "got {answer}",
    );
    assert_eq!(host.tool_names().len(), 4, "the host still serves");
}

/// The Channel is the host's: it opens, a WSS consumer sees its Open packet,
/// `ready` verifies it, and `close` closes it.
#[tested]
fn the_channel_handshake_completes() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let keys = generate_keys("host", &root.join("channel"));
    write_host_config(&root.join("host"), &keys, "");
    let host = Host::spawn_in(&root.join("scratch"), &[], &root.join("host"));
    let mut consumer = Consumer::attach(host, &keys.ca);
    let closed = consumer.host.interface("interface channel close");
    assert!(!has_error_path(&closed), "the host closes its Channel; got {closed}");
}

/// A sync Channel completes on the host: the first sync takes the greeting,
/// which claims the Channel, and `ready` verifies it.
#[tested]
fn a_sync_channel_completes() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());
    let opened = host.interface("interface channel open sync");
    assert_eq!(envelope(&opened)["result"]["status"], "new", "got {opened}");
    let synced = host.interface("interface channel sync");
    let packets = envelope(&synced)["result"]["packets"].clone();
    assert_eq!(
        packets[0]["model"], "mcp/channel/Open",
        "the first sync takes the greeting; got {synced}",
    );
    let ready = host.interface("interface channel ready");
    assert!(!has_error_path(&ready), "the claimed Channel verifies; got {ready}");
    let closed = host.interface("interface channel close");
    assert!(!has_error_path(&closed), "got {closed}");
}

/// The Channel's thresholds are the host's to read and adjust.
#[tested]
fn the_channel_thresholds_are_the_hosts() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn(t.temp_dir());
    let adjusted = host.interface("interface channel config --spam-warn-rate 7");
    assert_eq!(envelope(&adjusted)["result"]["spam_warn_rate"], 7, "got {adjusted}");
    let read = host.interface("interface channel config");
    assert_eq!(envelope(&read)["result"]["spam_warn_rate"], 7, "the change holds; got {read}");
}

/// `--test` selects the test namespace: the title and the instructions name it,
/// and the host serves on it.
#[tested]
fn the_test_flag_selects_the_namespace() {
    let t = testing::test!({ .using_temp_dir() });
    let log = t.temp_dir().join("host_stderr.log");
    let mut host = Host::spawn_uninitialized_args(
        &t.temp_dir().join("scratch"),
        &["--test", "--id", "serving"],
        &log,
    );
    let reply = host.initialize();
    let result = &reply["result"];
    assert_eq!(result["serverInfo"]["title"].as_str(), Some("grammar (test)"), "got {reply}");
    assert!(
        result["instructions"].as_str().is_some_and(|text| text.contains("namespace `test`")),
        "the instructions name the namespace; got {reply}",
    );
    let executed = host.execute(DEFINITION, "{x: 5}");
    assert_eq!(envelope(&executed)["result"]["out"], 6, "got {executed}");
    assert!(
        namespace_dir(host.data_home(), "serving", "test").is_dir(),
        "the test namespace holds the host's state",
    );
}
