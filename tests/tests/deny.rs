
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// The tool names `tools/list` advertises, sorted.
fn sorted_names(host: &mut Host) -> Vec<String> {
    let mut names = host.tool_names();
    names.sort();
    names
}

#[tested]
fn deny_removes_tools_from_list() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--deny", "execute,rerun"]);
    assert_eq!(
        sorted_names(&mut host),
        vec!["await", "bed", "call", "interface"],
        "the two denied are absent and the rest remain",
    );
}

#[tested]
fn denied_tool_call_fails_at_protocol_layer() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--deny", "execute"]);
    let resp = host.execute(
        "def execute [data: nothing]: nothing -> record<out: int> { { out: 1 } }",
        "null",
    );
    assert!(
        extract_envelope(&resp).is_none() && envelope(&resp).is_nothing(),
        "a denied tool must not execute, and answers no envelope at all; got {resp}",
    );
}

#[tested]
fn deny_interface_removes_the_tool_whole() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--deny", "interface"]);
    assert_eq!(sorted_names(&mut host), vec!["await", "bed", "call", "execute", "rerun"]);
    let resp = host.info();
    assert!(
        envelope(&resp).is_nothing(),
        "a denied interface answers no envelope at all; got {resp}",
    );
}

#[tested]
fn deny_every_other_tool_leaves_the_interface() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--deny", "execute,rerun,call,bed,await"]);
    assert_eq!(sorted_names(&mut host), vec!["interface"]);
    let info = host.info();
    assert!(!has_error_path(&info), "the interface still answers; got {info}");
}

/// A denied command leaves the parse, the description's list, and `interface help
/// interface`'s signatures; the tool and every other command stand.
#[tested]
fn a_denied_command_leaves_the_parse_and_the_description() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host =
        Host::spawn_args(t.temp_dir(), &["--deny", "purview configure,equip commit"]);
    let tools = host.tools();
    let interface = tools
        .iter()
        .find(|tool| tool["name"] == "interface")
        .unwrap_or_else(|| panic!("the interface stands; got {tools:?}"));
    let description = interface["description"].as_str().expect("its description");
    for denied in ["- purview configure\n", "- equip commit\n"] {
        assert!(!description.contains(denied), "{denied} is unlisted; got {description}");
    }
    assert!(description.contains("- purview list\n"), "got {description}");

    let help = host.interface("interface help interface");
    let block = envelope(&help)["result"]["doc"]["signatures"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    for denied in ["`purview configure [", "`equip commit ["] {
        assert!(!block.contains(denied), "{denied} is undocumented; got {help}");
    }
    assert!(block.contains("`purview list ["), "got {help}");

    let refused = host.interface("interface purview configure 'x' ['*']");
    assert_eq!(
        envelope_error_kind(&refused).as_deref(),
        Some("interface::invalid"),
        "a denied command is refused at the parse; got {refused}",
    );
    assert!(
        error_message(&refused).contains("no `interface` command"),
        "as a command the interface lacks; got {refused}",
    );
    let listed = host.interface("interface purview list");
    assert!(!has_error_path(&listed), "an undenied command runs; got {listed}");
    let bare = host.interface("purview list");
    assert!(!has_error_path(&bare), "a subcommand runs without its head; got {bare}");
}

#[tested]
fn deny_bed_removes_the_tool_whole() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--deny", "bed"]);
    assert_eq!(sorted_names(&mut host), vec!["await", "call", "execute", "interface", "rerun"]);
    let resp = host.bed("grmr", "config list");
    assert!(envelope(&resp).is_nothing(), "a denied bed answers no envelope at all; got {resp}");
}

/// A denied `grmr` command leaves `bed`'s description and is refused by `bed`,
/// every other one standing.
#[tested]
fn a_denied_grmr_command_leaves_bed() {
    let t = testing::test!({ .using_temp_dir() });
    let mut host = Host::spawn_args(t.temp_dir(), &["--deny", "grmr dbg"]);
    let tools = host.tools();
    let bed = tools
        .iter()
        .find(|tool| tool["name"] == "bed")
        .unwrap_or_else(|| panic!("bed stands; got {tools:?}"));
    let description = bed["description"].as_str().expect("its description");
    assert!(!description.contains("`grmr dbg ["), "got {description}");
    assert!(description.contains("`grmr config list ["), "got {description}");
    for subcommand in ["dbg {a: 1}", "grmr dbg {a: 1}"] {
        let refused = host.bed("grmr", subcommand);
        assert_eq!(envelope_error_kind(&refused).as_deref(), Some("bed::invalid"), "got {refused}");
        assert!(error_message(&refused).contains("withheld"), "got {refused}");
    }
    let config = host.bed("grmr", "config list");
    assert!(!has_error_path(&config), "an undenied grmr command runs; got {config}");
}

#[tested]
fn unknown_deny_token_fails_startup() {
    let t = testing::test!({ .using_temp_dir() });
    for token in ["bogus", "run", "interact", "nu", "purview_configure", "equip", "interface nosuch", "grmr nosuch"] {
        let out = run_output(t.temp_dir(), &["--deny", token]);
        assert!(
            !out.status.success(),
            "the unknown deny token `{token}` should fail startup; got {:?}",
            out.status,
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains(token),
            "the clap error should name the bad token; got {stderr:?}",
        );
    }
}
