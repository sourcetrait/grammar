# purview_configure.rs

## struct PurviewConfigureResult
The whole set, never a delta: the caller needs to check what it just wrote. A delta cannot
answer "what does this configuration actually do now", and against a purview the session is
not looking through - the common case - it answers nothing at all (several distinct writes
would all report an identical null delta).

## fn purview_configure
The write is the routed engine's (`crate/engine/src/equip/purview.rs`'s `purview_configure`),
which refuses the id and each reference and answers the rows after. The `@id` values are
validated for shape only: whether the referenced purview exists yet is the prune's
business, not the parser's, because `a -> @b` is legitimately written before `b`.

`default` cannot not exist, so an empty list resets it to `*` rather than deleting it - the
same state startup would put it back in.

A deletion must not leave the session pointing at something gone, which is what
`retain_known` is for, on the rows the write answered.
