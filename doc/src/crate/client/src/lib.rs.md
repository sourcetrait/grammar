# lib.rs

The MCP's side of its remote engines: a session with a worker over SSH, and
the system that holds one a host. It takes the protocol, russh, and
sourcetrait_common alone, never the engine, the operator, or the worker, so a
product that drives a worker links it without an engine of its own.

## mod endpoint
A remote engine's entry and its fields' checks, held before the remotes file
is written and before a session opens.

## mod session
The MCP's end of a session over any reader and writer, an SSH channel's or a
child's pipes, so the worker's own tests drive a real worker through the same
code the MCP does.

## mod ssh
The SSH connection a session rides, its exec channel, and the channel's
streams.

## mod system
The remote engines as a subsys system under the MCP's controller.

## deps
`bitcode`, `rkyv`, and `serde` are the cereal derive's own, every crate that
derives `Data` naming them.

## pub use
The endpoint, its defaults, and its fields' checks, the session, its events
and an ask's responder, the SSH open and its session, and the system with its
messages, a session's state, parameters, paths, and handle.
