# interface.rs

The `interface` tool: every tool but `execute`, `rerun`, `call`, and `bed`,
folded into one `subcommand` string (User's `grammar/interface`, its parameters
his `doc/human/tools.md`), the `interface` word optional and undocumented
(User). The commands convert those tools as they were: same parameters, same
results, same refusals. The parse is the interface crate's (`crate/interface`);
this host binds it to its deny set and routes what it parses.

## struct Caller
What a command may need of the request beyond its text: the client's name,
which chooses the face an async open takes (`channel_open.rs`), and the thread
`interface info` keeps for the Codex face (`face.rs`). It is read once from the
request's context, so the tool's body takes plain values that the in-process
harness and the one-shot CLI build without a client.

### fn Caller::of
rmcp's `client_info` reads the request's own `clientInfo` where the client
sends one per request, as the newer spec has it, and the `initialize` it kept
otherwise, so the server keeps no copy of its own.

## const INTERFACE_PRETEXT
User's `include/human/mcp/interface.md`, verbatim by `include_str!`: his pretext
and nothing else, the command list being the code's to render.

## fn interface_description
His pretext, its trailing whitespace trimmed so one blank line parts it from the
list whatever his file ends with, then the commands the deny set leaves, by
their words under their topics, rendered from the catalog the parse registers.
The signatures moved to `interface help interface` and the grammar-mcp skill,
keeping the description small as the commands grow (User).

## const INTERFACE_DESCRIPTION
The router's stub, which `interface_description` replaces on the router.

## fn interface_engine_for
The interface crate's engine, its commands the deny set leaves, the deny set
naming a command by its words, and the `await` tool's deny withholding `await
rig` too (`config.rs`'s `denies_interface`).

## fn ChannelFace::run_channel / run_peer / run_chat
The routing table's three groups on the face: the Channel's commands, the
remote channels', and the chat's run through the face that holds their systems.

## fn interface
The nonce is minted first, over the tool's name and the command text, so a
refusal at the parse carries one exactly as a dispatched command's answer does.
The host is routed before the parse (`NuSh::route`, `common.rs`), so an unknown
alias is refused first. The dispatched command answers under that same nonce:
each command's method takes the nonce rather than minting its own. The tool
takes the request's context beside its parameters and hands its body the
`Caller` it describes; `interface info` alone hands the face the thread it
names (`face.rs`).

## fn interface_as
The tool's body, its caller given: the server's tool, the in-process harness
(`guts.rs`), and the one-shot CLI (`server/oneshot.rs`) all run this.

## fn run_interface
Each group to its system: the face's two and the remote engines' whatever the
host, since the Channel, the remote channels, and the remote engines' entries
and sessions are the MCP's, and the engine's to the host's engine: the MCP's own
through `local_engine`, a remote engine's as the call's subcommand, which
grammar-ssh parses and runs (`crate/engine`'s `desk.rs`). The deny set applies
at the parse, before any routing, so a denied command is withheld the same for
every host. No command dispatches an eval, so none enters the in-flight table
and none takes a lane or a permit, `interface await status` aside, whose status
eval takes a permit as a beat's does; a command's own locks are taken inside its
method as its tool took them.

## fn run_remote_engine
The remote engines' group, each command its method on the MCP
(`remote_engines.rs`, `remote_engine_set.rs`, `remote_engine_connect.rs`, and
`remote_engine_import.rs`).

## fn mcp_block
`interface help interface` and `interface help grmr` answer the MCP's own blocks
whatever the host, the interface being the MCP's, its deny set applied, and grmr
the same on every engine; every other engine command goes where the call routes.

## fn local_engine
An engine command on the MCP's own engine, through the engine's handlers with
the MCP's desk (`common.rs`'s `LocalDesk`); the harness calls it with a parsed
command. The call's nonce goes with it, the `await` of a job an `await rig`
starts.

## fn remote_interface
A remote engine's call goes as its subcommand, the text the agent sent, and
comes back as its value or its rows: the wire carries the call, never the
command's values, so a command added or changed needs no change to the session
(User).
