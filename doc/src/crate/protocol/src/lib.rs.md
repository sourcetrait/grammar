# lib.rs

The crate is the portable side of a remote engine: typed data and the wire's
framing alone, with no evaluator, operator, MCP server, or platform monitoring
in its dependencies. The engine, the client, the worker, the MCP frontend, and
the Channel's, the peer links', and the interface's crates take it, and it
takes none of them, so the package graph stays one-way.

## mod channel
The Channel's reservations, caps, modes, phases, and state, and a send as
data, the eval's stop held back on its own side.

## mod codec
The wire's framing: the preface, the length-prefixed frames and their codec
over any byte stream, the pinned codec's identity, and the one door from a
payload to a value, a bounded decode.

## mod command
The `interface` tool's catalog, its commands, arguments, results, summaries, and
deny words, with both rendered blocks, shared by a frontend's description and
deny parsing and each command's dispatch.

## mod engine
The engine's requests, commands, and answers, with the diagnostics, their cap
and positions, the error, and the documents they carry, here so the MCP and a
worker share one definition; the engine re-exports them.

## mod error
The runtime's and the transport's errors, each rendering to the envelope's rows,
the transport's under stable `remote::engine::` kinds.

## pub mod fixture
The v1 fixture driver, public as a module of its own, so a native test decodes
the golden vectors by type through it.

## mod hello
A session's opening exchange, the negotiated bounds, and both ends' checks.

## mod limits
The frame ceilings and the decode budget.

## mod operator
The operator's portable data, its manager's observation and the emergencies it
classifies, at fixed widths; the operator's Linux monitoring stays in its crate.

## mod peer
The remote Channel's models, its commands' typed results, its lifecycle and
delivery events, a relayed message, and the rule a send's dest is held to.

## mod purview
The namespace's purviews, their rows and views, and the engine's requests of
them, here so a session carries them to a worker's engine.

## mod session
A session's frames after its hellos, both ways: the MCP's requests and the
answers to the worker's asks, the worker's responses, asks, and emergencies,
and each side's liveness and close.

## use nuval::wide
The wire's widening, shared with nuval so every count and offset widens one
way.

## use sourcetrait_common::{cereal, datum, subsys}
Common comes through the workspace's `sourcetrait_common` and its facade, as
every crate takes it (User): cereal for the data, `datum`'s nonce and nom for a
request's identity and its host, and `subsys`'s error for a reply of the wrong
kind. `cereal::derived` emits absolute `::serde`, `::bitcode`, and `::rkyv`
paths, so those three are the crate's own dependencies.

## mod tk
## mod tku
tokio's byte-stream traits and tokio-util's codec and bytes, under the names
the mcp crate gives them, for the framing's adapter alone; the data needs
neither.

## pub use
The framing, its ceilings, its errors, and the schema's ID; the hellos and the
session's frames; the interface catalog; the engine's protocol, its requests,
commands, and answers with their diagnostics, error, and documents; the
purviews; the value bounds and their checks; and nuval's data under its own
names, the value, its error and result, and every kind, so one value and error
representation serves every endpoint without a second hierarchy.
