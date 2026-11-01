# error.rs

ONE unified wire envelope for every tool-execution failure, built from typed Rust
values. No stringly-keyed disambiguation, no rendered-text middle layer, and no
JSON-RPC error codes for domain failures.

## enum GrammarMcpError
Stays the ergonomic typed value the impls build and bubble with `?`; it is not
serde-tagged on the wire. Two variants carry already-unified rows, `Engine` carries the
engine's error whole, its own violations' rows among them, and the rest are
single-condition. The rows are the engine's `Diagnostic`s (the engine's `error.rs`).

`From<io::Error>` blanket-converts to `Internal { phase: "io" }` so the tool impls can
use `?` freely rather than mapping at every filesystem call.

`From<EngineError>` wraps the engine's error whole: its kinds and messages are the
engine's, the equips' refusals and an eval's failure among them, as the wire has always
carried them. `From<TransportError>` wraps a transport failure whole under its
`daemon::` kind.

### fn as_runtime
Every variant but the engine's and the transport's is the runtime's portable
`RuntimeError`, whose kinds, messages, and rows are its own (the protocol crate's
`error.rs`): this host's variants stay the values its impls build and bubble, and
their wording lives once. `From<RuntimeError>` turns a worker's refusal back into
the same variant, so it reaches the envelope as this host's own did.

## fn error_to_call_result
The failure envelope, `{nonce, error}`: the error's rows as a table of `kind`,
`source`, and `message`, rendered through the same seam as a success
(`envelope_text`, `server/tool/common.rs`). A violations error carries its error rows
alone and drops its warnings: a refusal's table holds what refused it.

It takes anything that becomes a `GrammarMcpError`, so an engine refusal reaches the
envelope with no wrap at the call.

WHY SUCCESS-SHAPE ERRORS: in the Claude Code CLI every `is_error = true` result and
every JSON-RPC error renders a red bullet with NO body, so a failure flagged as one is a
failure the agent cannot read. The envelope rides a plain success result, and its
`error` field is what tells the two shapes apart.

Every tool mints its nonce before any check, so every failure carries one. For an eval
it names the call's log directory, where a failure that produced output before dying
left its stdout and stderr.

`Err(mcp::ErrorData)` stays reserved for genuine rmcp-boundary failures, an envelope
that will not render among them.
