# lib.rs

The manifest carries no prose of its own by convention, so what follows is only
what a reader of it could reasonably get wrong.

## mod engines
The remote engines' entries in the remotes file, which the MCP reads and
writes itself: the peer crate keeps the channels' entries and the file's
locked edit, which both take.

## mod server
One build: the server and the one-shot CLI run on the process's own engine,
the controller over the face's systems, the engine's, the remote engines', and
the operator's (User). The Channel's and the peer links' systems
(`crate/channel`, `crate/peer`) are the MCP's own, and the remote engines'
(`crate/client`) its sessions with its workers.

## use sourcetrait_grammar_scratch
The namespace lock is the scratch crate's, shared with grammard, so both
programs take one lock under one record's shape.

## mod schema
The shim re-exporting `schemars::JsonSchema`, reached only through the
`schema::JsonSchema` path on a derive.

## mod json
serde_json's `Value` and `to_value` for `guts` alone, whose tool list is the
JSON-RPC's own `tools/list`; nothing the host serves converts through JSON.

## use sourcetrait_grammar_engine
The engine's public face named bare, as mcp's own items are, and the crate
itself as `engine`, the path its `init` is called by.

## use futures_util
`SinkExt` and `StreamExt`, and `PemObject` beside them, ride at the crate root
rather than inside a shim. They are traits, so method dispatch needs them in
scope, and a shim would park them at `shim::Trait` where the calls in the
channel hub could not see them. That is the manifest's trait exception rather
than an oversight.

## mod guts
The one `pub mod` that is not part of the public face. The integration tests are
separate crates and therefore see only `pub` items, so the test-support harness
has to be public to be reachable at all; `guts` is the convention's name for
that, chosen to signal both its internal-only status and the bravery of anyone
outside the workspace reaching for it.

## pub use
The whole public surface: the CLI's parse, the deniable tools, the error, the
Channel's mode, which the error's Channel refusals carry, and the two entries,
`run_main` and `run_with`. Everything else is crate-internal.
