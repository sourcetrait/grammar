//! The `host` every tool takes: `local` is this MCP's own host, and any other
//! alias is refused as `host::unknown` before the tool does anything.

use sourcetrait_common::testing::prelude::*;
use sourcetrait_grammar_mcp::guts::{TestServer, error_kind, error_messages, nuon};

static TESTING: testing::ModuleWith<TestServer> = testing::module_with!(Integration, {
    .using_temp_dir()
    .setup(|_| TestServer::new())
});

const DEFINITION: &str = "def execute [data: nothing]: nothing -> record<ok: bool> { {ok: true} }";

#[test]
fn local_runs_on_this_host() {
    let s = TESTING.harness();
    let env = s.execute_on("local", DEFINITION, "null", None);
    assert_eq!(env["result"], nuon("{ok: true}"), "{env}");
}

/// The check precedes each tool's own: a rerun of no cached definition, a call of
/// no installed rig, and an interface command are each refused for their host first.
/// The alias is exact, so a case or a space makes another one.
#[test]
fn any_other_alias_is_unknown_on_every_tool() {
    let s = TESTING.harness();
    for alias in ["elsewhere", "Local", " local", ""] {
        for env in [
            s.execute_on(alias, DEFINITION, "null", None),
            s.rerun_on(alias, "NoSuchNonce1", "null", None),
            s.call_on(alias, "sourcetrait/ghost:m:noop", "null", None),
            s.interface_on(alias, "interface info"),
        ] {
            assert_eq!(error_kind(&env), Some("host::unknown"), "`{alias}`: {env}");
            assert!(
                env["nonce"].as_str().is_some_and(|nonce| !nonce.is_empty()),
                "a refusal carries its nonce; {env}",
            );
            assert_eq!(
                error_messages(&env),
                vec![format!(
                    "unknown host `{alias}`: `local` is this MCP's own host, and no other is configured"
                )],
            );
        }
    }
}
