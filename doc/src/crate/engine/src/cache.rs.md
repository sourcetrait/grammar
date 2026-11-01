# cache.rs

The call logs and the cached definitions, under the cache home the host gives the
engine (`params.rs`).

## enum CacheKind
The variant-to-dir-name mapping lives in ONE place so the per-call trees cannot drift
apart from the enum that selects them: `execute/` for every `execute` and `rerun` call, either
lane, and `calls/` for a rig's call.

## fn cache_dir
Composes the per-call dir but deliberately does NOT create it. The dispatch
prologue owns `create_dir_all` and passes the dir into the eval, so there is one
place that decides a call's artifacts exist.

## fn call_log_dir
The one place a nonce becomes a call's log dir at run time, for the host decls
that act for the running call. The nonce comes from `$env.NONCE`, which a body
can rewrite, so the base62 check lives HERE rather than at the tool boundary.
Both kinds are searched because the nonce alone does not say which surface made
the call, and a nonce names one call across all of them.

## fn remove_call_cache
A worker's session keeps no call past its end, so its end removes each call's
entries by nonce, in both kinds since the nonce alone doesn't say which made
it. The nonce is held to base62 first, the path-traversal defense, so a nonce
naming anything but one entry removes nothing.

## fn definition_file
Co-locating the definition with that call's stdout and stderr means ONE prune of
`execute/<nonce>/` reclaims logs and definition together - there is no separate id space
to reason about, because the nonce IS the rerun handle. The definition is saved as the
`.nu` source it is, readable and runnable as written.

`nonce` must be base62-validated by the caller. That check is the path-traversal
defense, and it lives at the tool boundary rather than here.
