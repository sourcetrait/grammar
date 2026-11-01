# handler.rs

## fn NuSh::tool_router
Deny is enforced at router assembly, not at call time, and that is the whole design: a
denied tool is absent from `tools/list`, so its schema never enters the agent's context and
there is no denied-error loop to fall into. A call anyway fails at the rmcp layer as an
unknown tool.

rmcp routers compose with `+` at runtime, which is what makes conditional assembly possible
at all rather than requiring a compile-time feature per tool.

Each of the four tools is deniable on its own, and there is no implication between tokens -
denying `execute` does not deny `rerun` - so an operator's config lists exactly what it
means. A denied interface command leaves the tool standing: the command leaves the
interface's parse and its description instead (`crate/interface`'s `grammar.rs`), and
denying `interface` removes the tool whole.

The interface's description is written here, after its router is built, because it is
rendered from the deny set at runtime and the tool attribute takes only a constant.

## const MCP_DOC
User's `include/human/mcp/mcp.md`, verbatim: the envelope every tool answers, which he
keeps as the contract's own text, so the instructions an agent reads are his schema
rather than a restatement of it.

## fn get_info
`ServerCapabilities::builder().enable_tools()` is required. `default()` advertises no tool
capability and Claude Code then skips `tools/list` entirely, which presents as a server with
no tools rather than as an error.

`env!("CARGO_PKG_VERSION")` must expand in this crate, so the version cannot be threaded in
from elsewhere.

The title suffixes a non-default namespace so co-running namespace variants stay
tellable-apart in the client's own listing.

`instructions` opens with `MCP_DOC`, then renders the cache and data paths live as this host
resolved them, rather than as a static string or as a variable an agent's own environment
might resolve differently, so what it names is what this host actually uses. A remote
engine's results name its own host's paths, which the instructions say once, so an agent
reads a remote call's paths on that host. The agent-facing operational craft beyond the
envelope and those paths lives in the skills, not here.

The same config answers `server/discover` in the 2026-07-28 era, rmcp building the
DiscoverResult from it, so a client that never sends `initialize` still gets the title and
the instructions.
