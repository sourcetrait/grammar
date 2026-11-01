# purview.rs

The namespace's purviews beside the equip store they filter: the stored rows,
their load and save, their seeding, their pruning, and their resolution. Every
write runs under the equip store's one lock, the default's seeding among them.
A controller reaches them through the engine's own requests (`msg.rs`'s
`PurviewRequest`); the session's current view stays the controller's.

## const META_DIR
The namespace's meta directory - where `purviews.nuon` lives, alongside `keypair/`,
`equip/` and `host.lock`.

## const PURVIEW_DEFAULT
There is no unconfigured default, which is the invariant the whole subsystem rests on.
See `ensure_default_purview`.

## const PURVIEW_CORE
The builtins together, each by a reference to its own purview, so removing one builtin's
purview takes it out of `core` with no edit to `core` itself.

## const PURVIEW_REF
`@` is unambiguous as a sigil because `is_valid_ident` never admits it, so no author,
rig, module or call can begin with one. That is what makes the reference form
non-colliding by construction rather than by convention.

## fn purviews_to_nuon
Through nuval's serde support, as the rig index is, so one conversion covers the whole
shape and the file can't drift from the struct as it changes. nuval's default style is
nushell's `to nuon`'s, so the table writes as it always has, byte for byte.

## fn store_failed
The kinds a store failure has always answered: an io failure as `internal` under
`io`, a table that won't decode or encode under its own phase.

## fn load_purviews
A decode failure is a loud error, never a silent empty: an empty purview table resolves to
"sees nothing" while looking configured, so a corrupt file read as empty would be a silent
misconfiguration rather than a fault the caller can see.

## fn save_purviews
The table is written to a staging file beside it and renamed over it, so a list or a
resolve, which read without the equip store's lock, sees the old table or the new one
whole, never a half-written file. The staging file's name takes the process and a count,
so two writers, another host on the namespace among them, never share one.

## static STAGED
The count behind each staging file's name.

## fn is_valid_purview_id
Path-like but never a path: a leading `/` or `./` is rejected outright, because a label
that looks like a filesystem path invites being read as one. A purview id is arbitrary,
unrelated to any namepath or file on disk.

## fn is_valid_purview_ref
`@.` and `@*` are refused. Both are derived rather than stored, so they name no row and
can reference nothing - leaving them legal would mean deciding whether they resolve to
everything or to nothing, and neither answer is defensible.

## fn expand_values
Reports stay raw; only the filter path expands. That is why this is separate from
`resolve_patterns` rather than folded into it: a caller that displays configuration
shows what was written, and a caller that matches against it expands first.

Cycles flatten rather than lock up. A purview already visited on this walk contributes
nothing the second time, so `a -> @b -> @a` terminates with the union of both and
`a -> @a` terminates with a's own patterns. Writing a cycle is legal; it simply cannot
buy anything on the revisit. No configure-time rejection is possible anyway - `a -> @b`
is legal until someone later sets `b -> @a`.

## fn ensure_default_purview
Materializing `default` once at startup makes "default has a row" an invariant rather
than a fallback every reader would have to remember, and everything downstream is
simpler for it: `resolve_patterns` has no absent-default arm, `is_nameable_purview`
special-cases only `*`, an equip install just appends to the rows it finds, and an
uninstall has no "is this namespace configured yet" question to ask.

It is also why `interface purview configure default []` resets to `['*']` rather than
deleting: `default` has no not-existing state, so the delete operation everywhere else
puts it back to what startup would have written. The substrate runs it before any
engine system starts, under the equip store's lock as every other write.

## fn seed_builtin
Seeds only what is absent: a row a user wrote keeps what it holds, so installing a builtin
again never undoes their configuration.

## fn seed_builtin_purviews
Runs once, for the builtins a new namespace installed: each one's own purview and its
reference in `core`, under the equip repository's lock the purview table rides. `default`
keeps the `['*']` a new namespace starts with.

## fn resolve_patterns
An id with no row contributes nothing rather than everything, because a typo must narrow
the view rather than silently open it.

## fn is_rig_pattern
Asked by parse rather than by string, so `gram:` and `sourcetrait/gram:` are one pattern:
an install never adds the second spelling beside the first, and an uninstall takes either.

## fn pattern_requires
The `@id` check comes before parsing, because `@id` is not a namepath and must never
reach the namepath grammar at all.

## fn prune_dangling
Pruning is by registration, not by emptiness: a rig with no calls yet still satisfies its
own pattern, and a purview pointing at it should survive until the rig actually goes
away.

The id set is taken before the mutable walk so a reference is judged against the whole
table - a purview referencing one defined beside it survives, and so does a cycle, since
both ends have rows.

A row pruned down to nothing is dropped rather than kept as an empty purview, because an
empty pattern list is already the delete operation on the configure tool. A surviving
empty row would be a state the tool surface cannot otherwise produce.

It runs until a pass drops nothing, since a dropped row leaves every reference to it
dangling: uninstalling a builtin empties its own purview, and the next pass takes its
`@` out of `core`.

## fn purview_configure
The id and every reference are refused before the lock is taken, in the order and the
words the command has always given; the write prunes what dangles and answers the rows
after, so the controller can keep its view to what exists.

## fn purview_resolve
One read answers what a view needs: the patterns to match, expanded; each id's own
patterns, raw, as `info` reports them; and the ids no purview answers to, in order, so a
view naming one is refused by the first.

## fn purview_add_rig
A freshly installed rig joins `default` and every purview the session has in view, a
builtin by `@core` rather than its own pattern; the session's view comes with the
request, since it is the controller's.

## fn purview_remove_rig
An uninstalled rig leaves every purview that named it, in either spelling, and the
rows after go back so the session drops an id that went with it.
