//! The tool surface's documentation: User's three texts, verbatim, and no tool
//! declaring an output schema.

use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_mcp::guts::TestServer;

static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

const EXECUTE_DOC: &str = include_str!("../../../include/human/mcp/execute.md");
const MCP_DOC: &str = include_str!("../../../include/human/mcp/mcp.md");
const INTERFACE_DOC: &str = include_str!("../../../include/human/mcp/interface.md");

fn description(s: &TestServer, name: &str) -> String {
    let tools = s.tools();
    let tool = tools
        .iter()
        .find(|tool| tool["name"] == name)
        .unwrap_or_else(|| panic!("`{name}` is not routed"));
    tool["description"].as_str().unwrap_or_default().to_string()
}

#[test]
fn the_execute_tool_is_described_by_its_contract() {
    let s = TESTING.harness();
    assert_eq!(description(s, "execute"), EXECUTE_DOC);
}

#[test]
fn rerun_names_the_tool_it_reruns() {
    let s = TESTING.harness();
    assert!(description(s, "rerun").contains("`execute`"));
}

#[test]
fn the_instructions_open_with_the_envelope_schema() {
    let s = TESTING.harness();
    let instructions = s.instructions();
    assert!(instructions.starts_with(MCP_DOC), "{instructions}");
    assert!(instructions.contains("/execute/<nonce>/definition.nu"), "{instructions}");
    assert!(instructions.contains("the paths in its results are that host's own"), "{instructions}");
}

#[test]
fn no_tool_declares_an_output_schema() {
    let s = TESTING.harness();
    for tool in s.tools() {
        assert!(tool.get("outputSchema").is_none(), "{tool}");
    }
}

#[test]
fn four_tools_are_routed() {
    let s = TESTING.harness();
    let mut names: Vec<String> = s
        .tools()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap_or_default().to_string())
        .collect();
    names.sort();
    assert_eq!(names, vec!["call", "execute", "interface", "rerun"]);
}

#[test]
fn the_interface_is_described_by_its_pretext_then_its_command_list() {
    let s = TESTING.harness();
    let description = description(s, "interface");
    let (pretext, list) = description
        .split_once("\n\n# Command Signatures\n")
        .unwrap_or_else(|| panic!("the list follows the pretext: {description}"));
    assert_eq!(pretext, INTERFACE_DOC.trim_end(), "{description}");
    for topic in ["General", "Process", "Purview", "Channel", "Remote", "Equip"] {
        assert!(list.contains(&format!("## {topic}\n")), "{topic}: {description}");
    }
    assert!(
        list.starts_with("## General\n- interface info\n- interface help\n"),
        "{description}",
    );
    assert!(
        list.contains("- interface channel open\n- interface channel sync\n"),
        "the sync follows the open; {description}",
    );
    assert!(list.contains("- interface remote channel listen\n"), "{description}");
    let commands = list.lines().filter(|line| line.starts_with("- interface ")).count();
    assert_eq!(commands, 26, "{description}");
    assert!(!list.contains('`'), "the list names commands alone; {description}");
}

/// The grammar-mcp skill's last section is `interface help interface`'s block, held
/// here and rewritten under `BLESS=1`.
#[test]
fn the_skill_ends_with_the_interface_signatures() {
    let s = TESTING.harness();
    let answer = s.interface("interface help interface");
    let block = answer["result"]["doc"]["signatures"]
        .as_str()
        .unwrap_or_else(|| panic!("`interface help interface` answers a block: {answer}"))
        .to_string();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../agent/skill/grammar-mcp/SKILL.md");
    let skill = std::fs::read_to_string(&path).expect("the skill");
    if std::env::var("BLESS").as_deref() == Ok("1") {
        let head = skill
            .split("\n## Command Signatures\n")
            .next()
            .unwrap_or_default()
            .trim_end();
        std::fs::write(&path, format!("{head}\n\n{block}")).expect("bless the skill");
        return;
    }
    assert!(
        skill.ends_with(&format!("\n\n{block}")),
        "the skill's last section is `interface help interface`; bless with BLESS=1:\n\
         --- block ---\n{block}--- end ---",
    );
}

#[test]
fn every_description_fits_claude_codes_cap() {
    let s = TESTING.harness();
    for tool in s.tools() {
        let description = tool["description"].as_str().unwrap_or_default();
        assert!(
            description.chars().count() <= 4096,
            "`{}` is {} characters",
            tool["name"],
            description.chars().count(),
        );
    }
}
