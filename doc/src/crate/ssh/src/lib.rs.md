# lib.rs

grammar-ssh, the grammar MCP on a foreign host (User): a `remote engine
connect` reaches it over SSH, its session's stdio the wire, and it serves that
connection for as long as it persists, until the MCP closes it or a connection
error ends it (User). Its code follows the MCP's, the MCP its reference: the
same files for the same concerns, `run.rs`, `cli.rs`, `config.rs`, `home.rs`,
`scratch.rs`, `error.rs`, `controller.rs`, `host.rs`, `operator.rs`, and
`emergency.rs`, with `session.rs` its MCP-facing session where the MCP has its
rmcp service. It is the host: it controls its session and holds one engine
instance, whose nom it inherits (User).

## mod guts
What the crate's integration tests name and nothing else uses: the config, the
homes, and the scratch, public for them alone.

## mod tk
tokio's names under one path, its stop events by platform: tokio's Unix
signals on Unix and its Windows console events on Windows, each the form
`session.rs`'s `Stops` takes there.

## pub use
grammar-ssh's entry, the config file's name, and the error with its kind.
