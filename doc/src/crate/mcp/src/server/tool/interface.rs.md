# interface.rs

The `interface` tool: every tool but `execute`, `rerun`, and `call`, folded into one
`command` string (User's `grammar/interface`, its parameters his `doc/human/tools.md`).
The commands convert those tools as they were: same parameters, same results,
same refusals. The parse is the interface crate's (`crate/interface`); this
host binds it to its deny set and routes what it parses.

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
naming a command by its words.

## fn ChannelFace::run_channel / fn ChannelFace::run_peer
The routing table's two groups on the face: the Channel's commands and the
remote channels' run through the face that holds their systems.

## fn interface
The nonce is minted first, over the tool's name and the command text, so a
refusal at the parse carries one exactly as a dispatched command's answer does.
The host is routed before the parse (`NuSh::route`, `common.rs`), so an unknown
alias is refused first. The dispatched command answers under that same nonce:
each command's method takes the nonce rather than minting its own.

## fn run_interface
Each group to its system: the face's two and the remote engines' whatever the
host, since the Channel, the remote channels, and the remote engines' entries
and sessions are the MCP's, and the engine's to `run_engine` on the host's
engine. The deny set applies at the parse, before any routing, so a
denied command is withheld the same for every host. No command dispatches an
eval, so none appears in `interface process list`'s table and none takes a
lane or a permit; a command's own locks are taken inside its method as its
tool took them.

## fn run_remote_engine
The remote engines' group, each command its method on the MCP
(`remote_engines.rs`, `remote_engine_set.rs`, `remote_engine_connect.rs`, and
`remote_engine_import.rs`).

## fn run_engine
The engine's group, each command its method on the routed engine, the
purviews', `info`'s, and `help`'s with the session's view there.
