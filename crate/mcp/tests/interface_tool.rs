//! The `interface` tool end to end through the in-process harness: every
//! command reached through its parse and dispatch, answering as its tool did,
//! and the refusals the contract makes.

use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_mcp::guts::{
    NuVal, TestServer, equip_source, error_kind, error_messages, has_error, nuon,
    valid_function_source, write_source,
};

static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

/// A string as a Nu literal, single-quoted; the tests' paths hold no quote.
fn quoted(text: &str) -> String {
    assert!(!text.contains('\''), "{text}");
    format!("'{text}'")
}

/// A successful envelope's result, panicking on an error envelope.
fn result_of(env: &NuVal) -> &NuVal {
    assert!(!has_error(env), "got {env}");
    &env["result"]
}

#[tested]
fn help_answers_the_interfaces_and_grmrs_signatures() {
    let s = TESTING.harness();

    let interface = s.interface("interface help interface");
    let block = result_of(&interface)["doc"]["signatures"].as_str().unwrap_or_default();
    assert!(
        block.starts_with("## Command Signatures\n### General\n"),
        "got {interface}",
    );
    assert!(
        block.contains("`remote channel listen [alias: string, --on: string]: nothing -> record`"),
        "got {interface}",
    );

    let grmr = s.interface("interface help grmr");
    let block = result_of(&grmr)["doc"]["signatures"].as_str().unwrap_or_default();
    assert!(block.starts_with("## grmr Signatures\n"), "got {grmr}");
    assert!(block.contains("`grmr channel send ["), "got {grmr}");
    assert!(block.contains("`grmr config list ["), "got {grmr}");
    let lines = block.lines().filter(|line| line.starts_with("`grmr ")).count();
    assert_eq!(lines, sourcetrait_grammar_engine::GRMR_COMMANDS.len(), "every grmr command; got {grmr}");
}

#[tested]
fn every_command_answers_through_the_interface() {
    let t = testing::test!({ .using_temp_dir() });
    let s = TESTING.harness();

    let info = s.interface("interface info");
    assert_eq!(result_of(&info)["name"], "grammar", "got {info}");
    assert!(info["nonce"].as_str().is_some(), "the interface call mints the nonce; got {info}");

    let listed = s.interface("interface process list");
    assert_eq!(result_of(&listed)["processes"], nuon("[]"), "got {listed}");
    let killed = s.interface("interface process kill 'noSuchNonce1'");
    assert!(result_of(&killed).is_nothing(), "got {killed}");

    let source = equip_source(t.temp_dir(), "sourcetrait/ifprobe");
    let source_text = quoted(&source.to_string_lossy());
    let created = s.interface(&format!("interface equip new {source_text}"));
    assert!(result_of(&created)["summary"]["created"].as_list().is_some(), "got {created}");
    let scaffolded = s.interface("interface equip scaffold ['sourcetrait/ifprobe:m:go']");
    assert!(result_of(&scaffolded)["created"].as_list().is_some(), "got {scaffolded}");
    write_source(
        &source.join("rig"),
        "m/go/mod.nu",
        &valid_function_source("x: int", "out: int", "{ out: ($data.x * 2) }"),
    );
    let committed = s.interface("interface equip commit 'sourcetrait/ifprobe'");
    assert!(result_of(&committed)["added"].as_list().is_some(), "got {committed}");
    assert_eq!(result_of(&committed)["warnings"], nuon("[]"), "got {committed}");
    let called = s.call("sourcetrait/ifprobe:m:go", "{x: 21}");
    assert_eq!(result_of(&called)["out"], 42, "got {called}");
    let checked = s.interface(&format!("interface equip check {source_text}"));
    assert_eq!(result_of(&checked)["summary"]["ok"], true, "got {checked}");
    let helped = s.interface("interface help 'sourcetrait/ifprobe:m:go'");
    assert_eq!(
        result_of(&helped)["doc"]["signature"],
        "sourcetrait/ifprobe:m:go <x:int> <out:int>",
        "got {helped}",
    );

    let configured = s.interface("interface purview configure 'iface/probe' ['sourcetrait/ifprobe:']");
    assert!(result_of(&configured)["signatures"].as_str().is_some(), "got {configured}");
    let extended = s.interface("interface purview extend ['iface/probe']");
    assert_eq!(
        result_of(&extended)["current"],
        nuon(r#"[default, "iface/probe"]"#),
        "got {extended}",
    );
    let set = s.interface("interface purview set []");
    assert!(!has_error(&set), "got {set}");
    let purviews = s.interface("interface purview list");
    assert_eq!(result_of(&purviews), &s.purviews()["result"], "got {purviews}");

    let uninstalled = s.interface(&format!("interface equip uninstall {source_text}"));
    assert_eq!(result_of(&uninstalled), &nuon("{}"), "got {uninstalled}");
    let installed = s.interface(&format!("interface equip install {source_text}"));
    assert!(result_of(&installed)["summary"]["added"].as_list().is_some(), "got {installed}");
    let _ = s.interface(&format!("interface equip uninstall {source_text}"));

    let config = s.interface("interface channel config");
    assert_eq!(result_of(&config), &s.config_channel(None, None, None, None)["result"]);
    let tuned = s.interface("interface channel config --spam-warn-rate 11");
    assert_eq!(result_of(&tuned)["spam_warn_rate"], 11, "got {tuned}");
    let _ = s.interface("interface channel config --spam-warn-rate 10");
    let ready = s.interface("interface channel ready");
    assert_eq!(error_kind(&ready), Some("channel::not_open"), "got {ready}");
    let closed = s.interface("interface channel close");
    assert!(result_of(&closed).is_nothing(), "got {closed}");

    let remotes = s.interface("interface remote channel list");
    assert_eq!(result_of(&remotes), &nuon("[]"), "got {remotes}");
    let connected = s.interface("interface remote channel connect 'nopeer'");
    assert_eq!(error_kind(&connected), Some("remote::channel::unverified"), "got {connected}");
    let missing = s.interface("interface remote channel get 'nopeer'");
    assert_eq!(error_kind(&missing), Some("remote::channel::invalid_params"), "got {missing}");
    let shut = s.interface("interface remote channel close 'nopeer'");
    assert_eq!(error_kind(&shut), Some("remote::channel::not_open"), "got {shut}");

    let unlinked = s.interface("interface chat send 'nopeer' 'a summary' 'a message'");
    assert_eq!(error_kind(&unlinked), Some("remote::channel::invalid_params"), "got {unlinked}");
    let long_summary = quoted(&"s".repeat(81));
    for summary in ["''", "' '", long_summary.as_str()] {
        let refused = s.interface(&format!("interface chat send 'nopeer' {summary} 'a message'"));
        let messages = error_messages(&refused).join(" ");
        assert!(messages.contains("a chat's summary"), "{summary} is refused for itself; got {refused}");
    }
    let unread = s.interface("interface chat read 'abc'");
    assert_eq!(error_kind(&unread), Some("channel::not_open"), "a read wants the open's inbox; got {unread}");
    let unsafe_id = s.interface("interface chat read '../abc'");
    assert_eq!(error_kind(&unsafe_id), Some("remote::channel::invalid_params"), "got {unsafe_id}");

    let engines = s.interface("interface remote engine list");
    assert_eq!(result_of(&engines), &nuon("[]"), "got {engines}");
    let made = s.interface("interface remote engine set 'lab' --address 'lab.example:2222'");
    assert!(result_of(&made).is_nothing(), "a set answers nothing; got {made}");
    let got = s.interface("interface remote engine get 'lab'");
    assert_eq!(result_of(&got)["configured"], true, "got {got}");
    assert_eq!(result_of(&got)["address"], "lab.example:2222", "got {got}");
    assert_eq!(result_of(&got)["state"], "closed", "got {got}");
    assert_eq!(result_of(&got)["missing"], nuon("[platform, user, key, known_hosts]"), "got {got}");
    let listed = s.interface("interface remote engine list");
    assert_eq!(result_of(&listed)[0], *result_of(&got), "got {listed}");
    let incomplete = s.interface("interface remote engine connect 'lab'");
    assert_eq!(error_kind(&incomplete), Some("remote::engine::invalid_params"), "got {incomplete}");
    let routed = s.interface_on("lab", "interface info");
    assert_eq!(error_kind(&routed), Some("remote::engine::invalid_params"), "got {routed}");
    let refused = s.interface("interface remote engine set 'lab' --address 'lab.example:0'");
    assert_eq!(error_kind(&refused), Some("remote::engine::invalid_params"), "got {refused}");
    let unset = s.interface("interface remote engine unset 'lab'");
    assert!(result_of(&unset).is_nothing(), "got {unset}");
    let emptied = s.interface("interface remote engine list");
    assert_eq!(result_of(&emptied), &nuon("[]"), "got {emptied}");
    let unknown = s.interface("interface remote engine get 'lab'");
    assert_eq!(error_kind(&unknown), Some("remote::engine::invalid_params"), "got {unknown}");
    let unopened = s.interface("interface remote engine connect 'lab'");
    assert_eq!(error_kind(&unopened), Some("remote::engine::invalid_params"), "got {unopened}");
    let unclosed = s.interface("interface remote engine close 'lab'");
    assert_eq!(error_kind(&unclosed), Some("remote::engine::not_open"), "got {unclosed}");
    let gone = s.interface("interface remote engine unset 'lab'");
    assert_eq!(error_kind(&gone), Some("remote::engine::invalid_params"), "got {gone}");
}

#[test]
fn a_subcommand_runs_with_or_without_its_head() {
    let s = TESTING.harness();
    for subcommand in ["info", "interface info"] {
        let info = s.interface(subcommand);
        assert_eq!(result_of(&info)["name"], "grammar", "`{subcommand}`: {info}");
    }
    let grmr = s.interface("help grmr");
    let block = result_of(&grmr)["doc"]["signatures"].as_str().unwrap_or_default();
    assert!(block.starts_with("## grmr Signatures\n"), "got {grmr}");
    let listed = s.interface("process list");
    assert!(result_of(&listed)["processes"].as_list().is_some(), "got {listed}");
    let refused = s.interface("channel config --spam-warn-rate -1");
    assert_eq!(error_kind(&refused), Some("interface::invalid"), "got {refused}");
    let position = refused["error"][0]["source"]["position"].clone();
    assert_eq!(position, nuon("[1, 33]"), "positioned in the subcommand as sent; got {refused}");
}

#[test]
fn a_refusal_names_the_contract_and_carries_the_nonce() {
    let s = TESTING.harness();
    for command in [
        "interface nosuch",
        "interface info | interface purview list",
        "interface info; interface purview list",
        "interface help $nu",
        "interface help (interface info)",
        "interface help",
        "interface channel config --spam-warn-rate -1",
        "ls",
    ] {
        let env = s.interface(command);
        assert_eq!(error_kind(&env), Some("interface::invalid"), "`{command}`: {env}");
        assert!(env["nonce"].as_str().is_some(), "`{command}`: {env}");
    }
}

#[test]
fn a_command_refusal_is_its_tools() {
    let s = TESTING.harness();
    let env = s.interface("interface help 'grammar:eye:nosuch'");
    assert!(has_error(&env), "got {env}");
    let unknown_equip = s.interface("interface equip commit 'acme/nosuch'");
    assert_eq!(error_kind(&unknown_equip), Some("equip::not_registered"), "got {unknown_equip}");
    assert_eq!(
        error_messages(&unknown_equip),
        vec!["equip `acme/nosuch` is not registered".to_string()],
    );
}
