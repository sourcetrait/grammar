# info.rs

`interface info` exists because the rmcp init handshake's `serverInfo` is not
relayed into agent context by Claude Code. The command surfaces the same facts
through `tools/call` - including the live callable surface, so no stale skill
index is needed.

## struct InfoResult

### field id
The agent's self-confirmation channel for which namespace it is talking to. The
same pair is readable ambiently inside a definition as `$env.EQUIP_ID` /
`$env.EQUIP_NAMESPACE`, so this is the outside view of the same fact.

### field nom
A changed value across two calls is how an agent learns the server restarted. A
pid cannot serve that, because pids are reused. It is `nom` here, the host's own
id in the host's own answer, and `mcp_nom` everywhere it names the host from
outside: a packet, the host lock, a definition's `$env.EQUIP_MCP_NOM` (User).

### field tmp
The outside view of `$env.EQUIP_TMP`, as `id` is of `$env.EQUIP_ID`: where the
agent itself reads the bulk a definition left in the scratch directory.

### field transient
The outside view of `$env.EQUIP_TRANSIENT`, the host's directory for small
runtime files.

### field nu_version
Baked by `build.rs` from the workspace lockfile's `nu-protocol` pin, so it
reports what is actually linked rather than what a manifest asked for.

### field signatures
Read straight from each rig's committed index - no `.nu` walk and no parse -
which is what makes the agent's first read after the skill cheap.

### field purview
Last in the struct because it is the frame around the block rather than part of
it.

## fn info
A purviews file that exists but will not decode is an error, never a silently
empty and therefore silently total view. The rig index already holds itself to
that rule and this follows it.

The current view alone: the command takes no argument, so what it renders is
what the session has in view, and `interface purview set` is how a view changes
(User).

The view is the session's, the block the engine's: the routed engine resolves
the session's view on it into patterns and its `Signatures` renders the rigs
within them.

A remote engine's info is its worker's: the version and the nushell its build
carries, its id, namespace, and nom, its workdir and scratch directories, and
its plugins, from the session's info request, so every path it names is that
host's. The name stays the MCP's, the one server the agent talks to.

Filtering expands `@` references while the reported `purview` field carries the
raw configuration - the two go through different functions on purpose, so a
report shows what was written and the filter matches what it means.
