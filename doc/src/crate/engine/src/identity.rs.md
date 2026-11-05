# identity.rs

An engine instance's identity: the nom the agent and its user see, and the
generator it is drawn from. Each controller, the MCP's and grammar-ssh's, holds
one engine instance and inherits its nom (User): the MCP's `interface info`,
its Channel's packets, its namespace lock's record, and its emergency log carry
it, as grammar-ssh's hello, its info, its lock's record, and its log do. The
end user talks to the engine through either host, their processes and
lifetimes opaque to it, so a controller shows no nom of its own (User).

## struct EngineIdentity
Drawn by its host before anything is made under its nom, the MCP's tree of
transient files among them, and carried in `EngineParams` for
`EngineHandle::start` to take (`system.rs`). A host process starts one engine
instance, so an engine started again after a failure is a new process, a
grammar-ssh its MCP restarts, under a new nom, and its controller's session
changes with it (User). The generator rides with the nom: the nonces a
controller mints come from the generator that drew its engine's nom.

### fn draw
The nom is the generator's first draw.
