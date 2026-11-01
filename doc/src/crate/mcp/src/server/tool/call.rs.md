# call.rs

## const CALL_DESCRIPTION
The tool's one line, as the router advertises it.

## fn call
The host is routed right after the nonce (`NuSh::route`, `common.rs`). The data and
the timeout cross to the engine as the agent's text, as `execute`'s do, so the request is
plain data whichever engine serves it and no host parses what the engine will check. The
routed engine resolves the namepath, takes the equip's lock, holds the data to `main`'s
contract, and runs the call (the engine's `serve.rs`), a worker on the rigs its own
namespace holds.

The result is strict, as `execute`'s is: the engine answers what `main`'s return type
holds or refuses it as `result::invalid`, so the render here meets only a value the
contract has already passed.
