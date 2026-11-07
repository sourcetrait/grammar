# desk.rs

The interface's engine commands, run where a call lands: the MCP's own engine for
a local call, grammar-ssh's for a call routed to a remote engine. The MCP sends a
remote engine's call on as its subcommand's text, and grammar-ssh parses it here
and runs it on its own engine, so the wire carries the tool call and its
parameters and never an interface command's values: a command added or changed
is the hosts' business alone. The answer is a value, or the rows its refusal
reports.

## trait Desk
What a host lends a command beside the parse: its engine, its identity, the
session's view, and its interface block. The MCP's is its own engine and config
(`crate/mcp/src/server/tool/common.rs`'s `LocalDesk`), grammar-ssh's its session's
(`crate/ssh/src/session.rs`'s `SessionDesk`).

### fn info
The host's identity, homes, and plugins, so `interface info` names the host the
call reached: every path in it that host's own.

### fn view
The session's view on this engine. The MCP holds one on its own engine, and each
grammar-ssh session one on its, fresh at the session's start.

### fn interface_signatures
`interface help interface`'s block, the host's deny set applied: the MCP's its
`--deny`'s, grammar-ssh's every command.

## struct CurrentPurview
Session-resident by design - it is a view rather than a configuration, so it belongs in
memory and dies with its session. Confirmed across a real restart, where the configured rows
persisted and the view reset.

### fn set
An empty list means `default`, so the view always names at least one purview and there is
no looking-at-nothing state to reason about - which is why there is no separate reset:
resetting is just setting the view to nothing in particular.

### fn retain_known
So a rig uninstall or a purview deletion cannot leave the session pointing at something
gone. The rows are the engine's answer to the write that changed them.

## struct InterfaceParser
The interface contract's engine, its commands alone, held so a host that parses a
subcommand names no nushell type.

## fn run_interface
Only an engine command runs where a call lands. The Channel's, the remote links',
the chat's, and the remote engines' commands are the MCP's, which runs them
whatever host a call names, so one reaching grammar-ssh is refused.

## fn run_engine_command
Each command on the desk's engine. A refusal is its rows, so a host answers its
caller without an error type of its own.

The await commands take a job by its await, the call's nonce its packets carry
(User): the list is the engine's table of the jobs it holds, the status the value
status answers once, and the stop the engine's stop by hand, answering nothing.
ENGINE-SCOPED: a job runs on the engine its call reached, so a remote engine's jobs
list on that host alone, and a one-shot CLI invocation lists none.

## struct InfoResult
`interface info` exists because the rmcp init handshake's `serverInfo` is not relayed into
agent context by Claude Code. The command surfaces the same facts through `tools/call` -
including the live callable surface, so no stale skill index is needed.

### field id
The agent's self-confirmation channel for which namespace it is talking to. The same pair
is readable ambiently inside a definition as `$env.EQUIP_ID` / `$env.EQUIP_NAMESPACE`, so
this is the outside view of the same fact.

### field nom
A changed value across two calls is how an agent learns the server restarted. A pid cannot
serve that, because pids are reused. It is `nom` here, the host's own id in the host's own
answer, and `mcp_nom` everywhere it names the host from outside: a packet, the host lock, a
definition's `$env.EQUIP_MCP_NOM` (User).

### field tmp
The outside view of `$env.EQUIP_TMP`, as `id` is of `$env.EQUIP_ID`: where the agent itself
reads the bulk a definition left in the scratch directory.

### field transient
A record, so a boot learns both of its transient directories at once: `grammar` the outside
view of `$env.EQUIP_TRANSIENT`, the host's directory for small runtime files, and `ai` an
agent's own scratch for the host's id. A remote engine's is worked out against that host's
runtime directory.

### field nu_version
Baked by `build.rs` from the workspace lockfile's `nu-protocol` pin, so it reports what is
actually linked rather than what a manifest asked for.

### field signatures
Read straight from each rig's committed index - no `.nu` walk and no parse - which is what
makes the agent's first read after the skill cheap.

### field purview
Last in the struct because it is the frame around the block rather than part of it.

## fn info
The current view alone: the command takes no argument, so what it renders is what the
session has in view, and `interface purview set` is how a view changes (User). The engine
resolves the view into patterns and its `Signatures` renders the rigs within them.

A purviews file that exists but will not decode is an error, never a silently empty and
therefore silently total view. The rig index already holds itself to that rule and this
follows it.

Filtering expands `@` references while the reported `purview` field carries the raw
configuration - the two go through different functions on purpose, so a report shows what
was written and the filter matches what it means.

## struct InspectResult
EXACTLY ONE FIELD. The namepath is not reprinted because the caller supplied it, and the
per-kind shapes carry nothing in common worth hoisting beside it.

## fn help
The two words come before any namepath: `interface` answers the interface's signature
block, the host's deny set applied, and `grmr` the grmr commands' (User). Both are held from
equip names (`equip/validate.rs`), so no namepath they could shadow exists.

CLASSIFICATION IS BY SHAPE AND HAPPENS FIRST. A trailing hierarchy character, or a bare `*`
or `.`, makes this a PATTERN; anything else validates as the exact namepath it always did,
with the exact parser's strictness unchanged.

`.` IS RESOLVED HERE, not in the parser, and that placement is the design: purview is the
only thing that knows what `.` names, so the parser leaves it unresolved and an unresolved
`.` matches nothing. Resolving it at this one call site is what makes `interface help .`
render the current view rather than an empty block.

An EXACT namepath is the engine's `Help`, read under its one equip's lock. A PATTERN SPANS
EQUIPS, so it renders as a signature block through the engine's `Signatures`, whose
renderer takes each equip's lock itself.

## struct PurviewsResult
`purviews` reports the configured table VERBATIM with `@` references unexpanded, because
this is a report rather than a filter - a reader here wants the configuration as written.

`current` is the KEYS alone, since `purviews` above already says what each one resolves to.
It lists what is CONFIGURED, which is not the same set as what is in view: an id can be
configured and not current, and `*` can be current without being a row.

## struct PurviewSetResult
ONE field. The caller just SET the view, so it needs telling neither what is in view nor
what left - only what it can now see.

`revealed` being null does NOT mean nothing changed: NARROWING reveals nothing new, and
neither does naming a purview whose rigs were already visible under another id.
`interface info` is the check for what is actually in view.

## fn purview_set
`@fae` and `fae` name the same purview, as they do for `interface info`, so a reference
copied out of a configuration works unchanged.

AN UNKNOWN ID IS REFUSED rather than silently dropped. For a command that REPLACES the view
a typo would otherwise narrow it to something the caller never asked for, which is
indistinguishable from the purview being empty.

## fn revealed
BOTH SIDES OF THE DELTA ARE EXPANDED before comparison, the engine's resolution of each
view. Comparing raw values would count `@ants` as new against a `sourcetrait/ant:` already
in view, and re-reveal a block the caller could already see.

## struct PurviewDeltaResult
The two halves are asymmetric on purpose: extending normally only adds, so a `hidden` field
would report an empty list that reads like a result.

`revealed` is `interface info`'s block rendered for the patterns that were newly added - not
a diff of two whole blocks. So when the view already carries `*`, what it shows was visible
before as well: the patterns are what changed, and the block says what they name.

## fn purview_extend
An unknown id is refused rather than ignored. Contributing nothing in silence would turn a
typo into a view that simply fails to widen, which looks identical to a purview that is
genuinely empty. The ids asked are resolved first, so the refusal names the first unknown
one before the view changes.

## struct PurviewConfigureResult
The whole set, never a delta: the caller needs to check what it just wrote. A delta cannot
answer "what does this configuration actually do now", and against a purview the session is
not looking through - the common case - it answers nothing at all (several distinct writes
would all report an identical null delta).

## fn purview_configure
The write is the engine's (`equip/purview.rs`'s `purview_configure`), which refuses the id
and each reference and answers the rows after. The `@id` values are validated for shape
only: whether the referenced purview exists yet is the prune's business, not the parser's,
because `a -> @b` is legitimately written before `b`.

`default` cannot not exist, so an empty list resets it to `*` rather than deleting it - the
same state startup would put it back in.

A deletion must not leave the session pointing at something gone, which is what
`retain_known` is for, on the rows the write answered.

## enum EquipSummary
Untagged: the caller knows the variant from the command it ran, and the shapes are
distinct. `uninstall` returns no summary at all - void, matching its idempotent success.

`install`'s summary is its commit's result, the validator's warning rows in its `warnings`
as `interface equip commit`'s carry them. `check` keeps both buckets, `errors` and
`warnings`: it is the validator's report, so an error there is a finding rather than a
refusal.

Each action runs on the engine the call reached, a source path being that host's own. The
install is the engine's and the view the session's, so the purview update follows the
engine's answer, on the same engine, outside the equip's lock (`serve.rs`). An uninstall
landing in between leaves nothing behind: the update's own prune judges by registration,
and drops the pattern it just added for a rig no longer there. An unknown equip's uninstall
returns success with no summary rather than an error, because absent is the requested
state.

The scaffold is the engine's `EquipScaffold`, all-or-nothing on pre-existence there; the
command answers every path the batch made. The commit is the engine's `EquipCommit`: a
builtin's bare name resolved, the equip's write lock taken, the source validated on a
current engine, its answer the commit's own result. A remote engine commits into its own
namespace, so its calls reach the rig and the MCP's don't.

## fn purview_rig
The rows are the engine's to adjust (`equip/purview.rs`'s `purview_add_rig` and
`purview_remove_rig`), the session's view going with the request, since a rig installed
joins each purview in view. An install brings a rig into view only when the equip has one;
a lib alone has no calls for a purview to show. An uninstall's answer is the rows after, so
the session drops any purview the prune removed, as a purview deletion does.

Non-fatal: the install or uninstall already succeeded and is not rolled back, so a
bookkeeping failure is reported to stderr rather than turned into a failed command. It is
worth reading though, because a `default` that failed to gain the pattern leaves the new rig
installed but out of view.
