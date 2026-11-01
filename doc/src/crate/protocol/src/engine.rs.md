# engine.rs

The engine's protocol: what a host asks of it and what it answers, with the
diagnostics, the error, and the documents an answer carries. Every type is plain
data (`cereal`'s `Data`), serializable whole, which is what lets a request name a
linked host's engine at all, and none of it needs the evaluator: the frontend and
the daemon share this one definition, and the engine re-exports it.

A request's `nonce` is the operation's public identity, the one its envelope and
its agent carry. subsys correlates a request with its reply by a message ID of
its own, a process-local counter that never reaches the wire; neither stands in
for the other.

## const BUILTINS
The equips central to the engine, called by their bare names: `gram:log:latest`
rather than `sourcetrait/gram:log:latest`. The list is closed and short by design,
so a builtin cannot shadow an equip anyone installs: every other head still carries
its author, and an equip named `gram` by another author stays reachable by its
full name.

## fn display_equip
The one place a builtin's full name turns back into its bare one, so the block, a
standalone signature, the pattern an install writes, and an error's message all
show the name an agent calls.

## enum Host
`Local` is the engine in this process; `Remote` a linked host by its nom, the
identity its Channel and its remote links already go by.

## struct ExecuteRequest
The parameters as the agent sent them, the data and the timeout still text: the
engine holds them to the contract, so a host never parses what the engine will
check.

## struct CallRequest
As `ExecuteRequest`, the data and the timeout still text: the engine holds them to
the contract its rig index records for the function's `main`.

## enum EngineCommand
Everything a host asks that isn't an eval, so one request type reaches the
engine's state: its calls in flight, its health, its equips, and its
documentation.

## struct ValueAnswer
The nonce rides back with the value or the refusal: it is the envelope's either
way.

## enum ControlAnswer
`Uninstalled` carries no equip when none was installed, and that is a success:
absent is the state asked for.

## struct ProcessRow
`source_nonce` and `path` are per-kind, absent where the kind has none, so the row
a host renders says by its shape which tool made the call.

## enum Lane
The two lanes an eval runs on: the stateless pool's clones and the stateful
engine's one thread. A definition's `def --env` picks the stateful lane; the host
records the lane with each call in flight and with each hung thread it watches.

## struct HungEval
A watched thread as the supervisor needs it: its lane tells a held stateless
permit from a stuck stateful lane, and its cancel time is what the grace is
measured from.

## struct CallDoc
There is NO `summary` field, and its absence is deliberate: SUMMARY IS PART OF THE
SIGNATURE, carried as the trailing ` # ...` exactly as the block does it.

## enum InspectDoc
UNTAGGED, so each member serializes as its own bare shape under the result's
`doc`. The caller always knows which member it will get, because it knows whether
it passed a pattern or an exact namepath.

The four shapes stay distinguishable by FIELD SET alone: `srcdir` marks a rig,
`src` plus `summary` a module, `src` plus `signature` a call, and a lone
`signatures` a pattern.

## struct Source
`path` is equip-repo-relative (`<equip>/<kind>/<file>`) for a validation
diagnostic and NULL for an `execute` definition's diagnostic, because that one is
located in the definition and there is no file to name. `position` is 1-based, and
`[0, 0]` means file-level rather than line 0.

Both fields serialize as PRESENT, with `path` as null when absent rather than
omitted. The whole `Source` is null on the `Diagnostic` for a non-located
condition - an eval timeout, an unregistered-equip error.

The shape stays flat - `path` plus `position` - which is what the agent consumes
directly, rather than a nested source-carrier the reader would have to unwrap.

`position` is a pair of u64s, the wire's fixed width, built from the parser's
usize line and column through nuval's `wide`, which never alters one: the same
diagnostic reads the same from any platform's worker.

## fn span_to_line_col
A row's position as the agent reads its own text, 1-based. It lives beside the
row, so the engine and the frontend's interface parse position their rows one
way.

## const LINT_VIOLATION_CAP
The most rows one refusal reports: the lint's, the validator's, and the
interface parse's, so the client caps its refusals with no engine.

## struct Diagnostic
The single type for both a validation finding and a body-lint finding - one shape
the wire and the check summary both carry.

WHERE A ROW RIDES CONVEYS ITS SEVERITY, which is why `severity` is
`#[serde(skip)]` and absent from the envelope serde writes: an error row in a
failure's `error` table, a warning row in the `warnings` of the result that raised
it, an `interface equip` commit's, install's, or check's. It is an internal
partition key there. bitcode's derive carries it, so a row crossing the daemon's
wire keeps its severity.

Per-condition data (`timeout_ms`, names, reason) lives IN `message`, not a typed
per-variant field: it is text either way, and a per-variant field would vary its
shape by kind.

## enum EngineError
Everything an eval reports is prose bound for `thread::returned_error`, with ONE
exception: a module-resolution cycle is a CONDITION an agent can route on, so it
reaches the envelope as its own kind rather than as a string that happens to read a
certain way. `Init` is the engine's start, which its host reports and exits on.

cereal data, so a refusal crosses as data where the engine is reached as a
component; mcp's `GrammarMcpError` carries it whole, its kind and message the
engine's, as the wire has always carried them.

The equips' refusals are the engine's too. REGISTRATION is `equip::*`, because the
unit installed, committed, and addressed by a source path is the equip; a rig's
tree rules keep `rig::*` and a lib's are `lib::*`. `equip::no_rig` stays apart
from `equip::not_registered`: the one is an installed equip asked for calls it
does not have, the other a name nothing answers to.

An `execute` call's refusals each name their part of the contract:
`definition::invalid` rows, gathered with the lint's under
`DefinitionViolations`, for the definition; `data::invalid` for data that isn't
NUON or doesn't conform; `timeout::invalid` for a timeout that isn't a positive
duration; and `result::invalid` for a value the definition's return type refuses.
Each message carries the path or the position, so the agent fixes the one thing
named.

`host::unserved` refuses a request naming a host whose engine this process doesn't
serve, before anything of it runs.

`module::circular_import` is a TOP-LEVEL namespace rather than a `rig::` kind,
because a resolution cycle is not a rig-structure rule even when the validator is
what finds it. The validator already emits non-`rig::` kinds,
`lint::summary_length` among them.

`From<String>` is what keeps every ordinary `?` site in the eval untouched by that
distinction, and `From<io::Error>`, `Internal { phase: "io" }`, every filesystem
call in the equips' lifecycle.

### fn message
An equip is named as every other surface names it, through `display_equip`: a
builtin by its bare name, `grammar:eye` rather than the `sourcetrait/grammar:eye`
the namepath resolved to, which no agent spells. `Init` reads as its reason alone,
the startup message the host prints.

### fn rows
A violations error's rows are its errors alone, its warnings dropped: a refusal's
table holds what refused it. `kind` and `message` serve a single-condition error
and never a violations error, which renders through its rows; `Display` joins
those rows' messages.

## enum ToEngineSys
The system's inbound messages. Each request type converts into its variant by
`From`, and its answer type converts back out of `FromEngineSys` by `TryFrom`, an
answer of the other variant subsys's `ResponseType`, its error for a reply of the
wrong kind. Which answer type a request names is the engine's, in its
`subsys::Request` impls.
