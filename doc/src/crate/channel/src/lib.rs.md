# lib.rs

The Channel as a subsys system: the logical Channel's state, its verification
gate, its sync queue, its spam policy, its inbox's files, and its packets'
rendering. The agent-facing hub is no part of it: each server's face binds the
WSS listener and pumps the lines this system sends up into it
(`crate/mcp/src/server/channel/`).

## use sourcetrait_common::cereal
The system's messages are cereal data, so the crate takes `::serde`,
`::bitcode`, and `::rkyv` as dependencies of its own, the paths
`cereal::derived` emits, as the engine and the operator do.

## use sourcetrait_grammar_protocol
The Channel's portable data, the frozen v1 protocol's: its phases and modes, the
epoch, the state, a body's send, the thresholds, the caps and the reservations,
and the spam pair's emergencies. The system's own requests and answers wrap
them, never adding a wire variant.

## pub use
What a controller names: the system, its handle, its parameters and paths; each
request and answer; the rendering a face's own packets need, its greeting among
them; and the spam thresholds a config carries. The state, the counter, and the
sink are the crate's own.
