# lib.rs

The peer links as a subsys system: the remote links' two-connection mTLS
transport, their registries, the remotes file, and a body's sends. What a link
receives goes up to the system's controller, which hands each relay and report
to the Channel (`crate/mcp/src/server/channel/face.rs`); the system never calls
the Channel's.

## use sourcetrait_common::cereal
The system's messages are cereal data, so the crate takes `::serde`,
`::bitcode`, and `::rkyv` as dependencies of its own, the paths
`cereal::derived` emits.

## use sourcetrait_grammar_protocol
The remote Channel's portable data, the frozen v1 protocol's: a command's
result and its refusal, a link's lifecycle and a send's delivery as events, and
a peer's message as a relayed packet. The system's requests wrap them.

## use sourcetrait_grammar_channel
The rendering a send's spill and its id take, the same functions the Channel
renders its own packets with, so a remote spill reads as a local one.

## pub use
What a controller names: the system, its handle, its parameters and paths;
each request and answer; the remotes file's name, which the config sits beside;
and `safe_dest`, which a sender's attachments are held to before they are read.
The links, the codec, and the verifier are the crate's own.
