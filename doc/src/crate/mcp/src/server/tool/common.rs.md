# common.rs

## struct NuSh
The server, one instance per process. The rmcp `#[tool_handler]` dispatches off
`self.tool_router`, which is assembled deny-filtered at construction.

### field controller
Every eval, every equip action, and every signature block is the engine's answer to a
request sent through mcp's controller (`server/controller.rs`), the same controller that
observes the engine for the operator and stops both; the Channel's commands and the
remote channels' run on its face (`server/channel/face.rs`). The controller is shared
with the signal sweep.

### field interface_engine
Built once, from the deny set `NuSh::new` is handed, so a denied command is unknown to the
parse for the host's whole life; nothing evaluates on it, so it is shared rather than
cloned per call.

### field view
The session's current purview on the MCP's own engine; a remote engine's lives with its
grammar-ssh session.

## fn NuSh::new
The deny set is a parameter rather than read from the config, because the one-shot CLI
builds its interface with none: deny gates an agent's surface, and the operator's path
keeps every command. The tool router takes the config's deny set either way.

The `mcp_nom` is the face's, the process's one (`process_identity`), since the
operator and the emergency log are keyed by it too.

## fn NuSh::mint
`mint_nonce` on the server's generator (`server/identity.rs`).
The request carries the nonce to the engine, which runs the call under it.

## struct LocalDesk
The MCP's own engine lent to the interface's engine commands (`crate/engine`'s
`desk.rs`): its engine, its identity from its config and homes, the session's view on
it, and the interface block its deny set leaves.

## enum Route
Where a tool's `host` sends its request: the MCP's own engine, or a configured
remote engine by its alias.

## fn NuSh::route
Every tool takes `host`, required, User's ruling: an agent working beside remote hosts,
possibly several, names where each call runs rather than leaving it to a default it might
assume wrongly. Each tool routes it right after minting its nonce, so an unknown alias
is refused, nonce and all, before the tool's own checks. `local` is the MCP's engine and
each `[[engine]]` alias in the remotes file a remote one, the file read at each call
(`engines.rs`), so an entry set or unset holds from the next call. An alias the file
lacks is `host::unknown`; an entry still missing a field is refused as
`remote::engine::invalid_params`, naming what it lacks. The match is exact, so a case
or a space makes another alias.

## fn NuSh::value
`execute`, `rerun`, `call`, and `bed` share it. The request carries `Host::Local` either
way: grammar-ssh's engine is local to itself, and the MCP routes before any engine sees
the request. A remote request goes to the remote engines' system as the tool call it is
(`server/controller.rs`), and grammar-ssh answers its value or the rows its refusal
reports; a refusal grammar-ssh made before its engine took the request, and the
transport's errors, answer under their `remote::engine::` kinds.

## fn answer_result
A call's answer as its envelope, the value or the rows, for an interface call run on
the MCP's own engine or answered by grammar-ssh.

## fn envelope_text
One seam for every envelope, a success's here and a failure's through
`error_to_call_result` (error.rs): a nu record, `nonce` first, rendered by `render_nuon`
as the result's one text content. There is no structured content and no output schema;
the text is the result, NUON the agent reads as it reads its own data.

## fn success
A tool's own result struct crosses to nu through `serde_to_nu`: serde writes it as a
`NuVal`, nuval's, rebuilt as the nushell value the envelope renders, with no JSON
between. `execute`, `rerun`, and `call` hand their checked value to `success_value` as
it is, so a duration stays a duration. A tool answering nothing carries `{nonce, result: null}`.

## fn answer_value
An eval's value crosses back as a `NuVal` and is rebuilt here into the nushell value the
envelope renders, a local engine's and grammar-ssh's alike. A request the engine's system
could not answer and a refusal the engine made fold into one error, which the tool's
envelope carries either way.
