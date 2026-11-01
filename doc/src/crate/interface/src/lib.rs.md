# lib.rs

The interface commands' one parser, which any product that serves the
`interface` tool takes: the isolated engine its parse runs on, the contract a
command is held to, and the parameters each command carries. The catalog of
commands is the protocol crate's (`COMMANDS`); what runs a command is its
controller's, routed by the system that owns it (`contract.rs`'s
`InterfaceCommand`).

## mod nu
nushell's parser and the protocol's engine types alone: the parse never
evaluates, so the crate takes no evaluating engine of its own. nu-engine is a
dependency for the feature pairing its manifest notes, never for its code.

## pub use
The parse and its engine, the command's four groups, the refusal's kind, and
each command's parameters. The literal arguments and the decls are the crate's
own.
