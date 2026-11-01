# serve.rs

The engine's answers to its requests. Each runs on a task of its own (`system.rs`), so a
request waiting on a permit, a lane, or an equip's lock holds no other request up.

## struct EngineCore
Built once, when the system starts. `CRYPTO_PROVIDER.default()` installs nushell's TLS
provider so the `http` family works; nushell reads its own OnceLock here, never rustls's
process-global, which is why this call rather than a rustls one.

The equips' locks are hydrated from the equip store at the start, so the store, and the
builtins it holds, are the host's to make before the engine starts: an equip installed
into the store behind the engine's back has no lock, and a call to it answers
`equip::not_registered`.

### field host
The system's link to its controller, held for every eval the core dispatches, each
lane handing it to its eval's decls.

## fn local
A request names its host, so one request type serves a linked host's engine as well as
this one. Until this engine serves a linked host's requests, a request naming one is
refused as `host::unserved` rather than run here, where it would answer from the wrong
machine.

## fn parse_timeout
The call's timeout and nothing else: parsed as NUON, which reads a duration literal, and
held to a positive duration. It is converted to whole milliseconds rounded up, so a
sub-millisecond duration still gets one. Its value goes on to a definition's or a main's
`timeout?` when its signature takes one, so a definition can budget against what the
host allows it. 120sec when absent, uncapped as every tool's timeout is.

## fn prepare_execute
The order is the contract's: the definition first, since its signature is what the data
is checked against; then the data; then the timeout. Every refusal comes before any log
directory exists.

A rerun passes `lint` false: its definition is the engine's own cache of a definition the
lint already passed.

## fn hold_data
One reading of the data for `execute`, a rerun, and a call alike. The text is parsed by
`from_nuon`, which evaluates constants alone and refuses a pipeline, then held to the
declared type by the strict check, then rendered back by `to_nuon` as the source's
literal. The literal is therefore the engine's own render of a plain value, never the
agent's text: nothing the agent wrote reaches an `execute` source but the definition,
which the contract has already held to one def, and nothing at all reaches a call's.

A text that doesn't parse is refused in the parse error's own words: the nuon crate's
error is a generic line, "error when parsing nuon text", with the parse error inside it,
so it is rendered by nuval's `short`, which carries the inner error, as `parse_timeout`
renders a timeout's. An empty text is refused rather than read as null, since a
`data: nothing` signature takes `null` and the field is never optional.

## fn hold_timeout
The timeout always bounds the eval; its literal goes into the source only when the
signature takes `timeout?`, since nushell refuses an argument a signature has no place
for.

## fn request_args
What the process list shows of an `execute` call, a rerun, or a call: the data and the
timeout as the agent sent them, its own text, so an agent matches an entry against what
it sent.

## fn run_prepared
The lane is the definition's: `def execute` on the stateless pool, `def --env execute` on
the stateful thread, so the agent picks persistence in the language it writes rather
than by choosing a tool. Both dispatches cache the definition beside the call's logs
before the eval, so a call that timed out can still be rerun with a larger timeout.

The result check runs on the eval's value before anything renders it: nushell checks no
output when a command runs, so a definition declaring `record<a: int>` and returning a
list, a record with an extra field, or nothing at all would otherwise succeed. A mismatch
is `result::invalid`; the eval's side effects stand, nothing being rolled back.

A value that passes crosses to the host as a `NuVal`, which keeps every plain kind, so a
duration stays a duration and a filesize a filesize when the host renders it.

## fn serve_rerun
`execute` again on an earlier call's definition with fresh data.

The `is_base62` gate is path-traversal defense in depth. The nonce is joined onto a
filesystem path, so it must reject `..`, `/` and anything outside `[0-9a-zA-Z]` before
that join happens.

The definition is analyzed again, never trusted as cached: its types are what the fresh
data and the result are held to, and the lane is its own `--env`. No re-lint: the
original call already linted it, and the cache is the engine's, so a lint could only
refuse something the engine itself wrote.

The rerun runs under its own nonce, the request's, and caches the definition again under
it, so every `execute` call's nonce, a rerun's included, is uniformly a handle, and the
calls' logs stay apart.

`InFlightKind::Rerun` carries the source nonce so the process list can show which
definition a rerun is re-firing, the one thing a caller cannot infer from the fresh nonce.

## fn serve_call
The path the template composes from the namepath is checked twice before it exists: the
ident rules in `Namepath::validate`, and the index as the authority, callability checked
against `.meta/rig.nuon` via `index_node` and the function's name. So the composed path,
absolute under the namespace's equip root (`template.rs`), names a call a commit wrote,
never one the namepath alone could steer.

The equip's read lock is held across the whole dispatch, so a concurrent commit cannot
swap the canonical tree out from under a running call. A lib-only equip answers
`equip::no_rig` under that lock, before the index is read.

The contract is execute's. The index's entry for the function carries the data and
result typedefs the validator read from `main`'s signature at commit, so the data is held
to its type before the dispatch and the value to its type after, `data::invalid` and
`result::invalid` as `execute` answers them, with no parse of the rig per call. A main
taking `timeout?` receives the call's timeout as a definition does.

No definition is cached: call targets are not rerunnable, because the source is the
committed rig's own and the handle would point at something the agent does not own.

Arity is checked here rather than in the parser: `validate` happily returns a Rig or
Module namepath, and only a call knows that it requires a Function.

## fn processes
Engine-scoped. The map is this engine's, so a one-shot CLI's own engine reports nothing in
flight rather than reaching into another host's.

The rows are cloned out under the map's lock, and the host renders them after it is
released, so the map is never held across a render.

## fn kill
Cooperative cancel: flipping the eval's interrupt `Signals` makes it bail at nushell's next
check point and free its permit. A hung thread - a pure-Rust loop that never polls - cannot
be reached, which is the accepted residual; its external children are reaped by the
teardown regardless.

An unknown or already-finished nonce is a race-safe no-op, because the dispatch's cleanup
guard removes the entry.

The escalation happens outside the map's lock, deliberately: both the process table's
walk and the SIGKILL are blocking, and holding the async mutex across them would stall
every other request.

`kill_plugin_subprocesses` fires here but never on an automatic timeout. It is broad -
nushell shares plugin subprocesses across evals - so it is reserved for a call the agent
explicitly killed, where unblocking a plugin-hung eval is worth restarting other evals'
plugins.

The hang watch is registered while the entry is still live, since the kill answers before
the dispatch future does and the entry would otherwise be gone by the time a hang could be
confirmed.

## fn health
The supervisor's one reading of the engine: the threads watched for a hang, the permits
held against the cap, and the jobs table's size, read together so the three describe one
moment. Confirming a hang, its grace, and what to raise are the supervisor's.

## fn equip_new
Registers the lock first and unregisters on failure, as `equip_install` does. That
ordering is what makes "already registered" a lock-level answer rather than a filesystem
race, and it is why both error paths call `unregister` explicitly.

## fn equip_install
The answer names the equip, so the host can bring a rig the install made into its own view.

## fn equip_uninstall
An unknown equip answers with no equip rather than an error, because absent is the
requested state.

The guard is dropped explicitly before `unregister`: unregistering while still holding the
lock would drop the map entry the guard belongs to.

## fn equip_scaffold
A bare equip namepath is rejected with a message pointing at `interface equip new`. This
command scaffolds modules and functions into an established equip's rig; establishing an
equip is `interface equip new`'s job, so an author reaching for it that way needs the other
command named rather than a generic arity complaint.

The batch is all-or-nothing on pre-existence. Every target equip is locked in canonical
sorted, deduped order first - sorted because a consistent lock order is what prevents a
deadlock between two concurrent batches - then every leaf is checked for pre-existence
across the whole batch before any is created. Otherwise a batch could half-apply and leave
the author to work out which half.

The scaffolding itself is not transactional beyond that check: a filesystem failure
mid-batch leaves what it already wrote.

## fn equip_commit
A builtin's bare name resolves to its equip first, so `interface equip commit gram` commits
`sourcetrait/gram`.

Takes the equip's write lock, since a commit wipes and rebuilds the canonical subtree, both
kinds of it, and a concurrent call reading the rig's index would see it mid-flight.

The engine comes from `lint_engine.current()` rather than a stored handle, so the validator
picks up a plugin registered since the host started. A validator that never refreshed would
reject source that runs.

The result is the commit's own, its changed paths and the validator's `warnings`; a
refusal's error table carries its errors alone.

## fn signatures
A pattern that no longer parses is treated as absent rather than fatal: the patterns come
from the host's purviews, agent-authored data, and one bad row must not take
`interface info` down with it.

## fn help
An exact namepath's documentation, read under its equip's read lock so a concurrent commit
cannot swap the docs out mid-read. A pattern spans equips, so its block is `signatures`,
whose renderer takes each equip's lock itself.
