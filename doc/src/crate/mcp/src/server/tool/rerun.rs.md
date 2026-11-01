# rerun.rs

## const RERUN_DESCRIPTION
The tool's one line, as the router advertises it.

## fn rerun
`execute` again on an earlier call's definition with fresh data: the same parameters as `execute`
less the definition, which the nonce names.

A FRESH nonce is minted for the rerun and the earlier one rides in the request as its
source; the host is routed next (`NuSh::route`, `common.rs`); the routed engine gates the
source, reads its cached definition, and runs it under the fresh nonce (the engine's
`serve.rs`). A worker caches the definitions of its own session's calls alone, so a
rerun names a call the same host ran in its session.
