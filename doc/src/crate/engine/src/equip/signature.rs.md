# signature.rs

## fn read_summary
A summary is single-line in EFFECT but is hard-wrapped in SOURCE like any other comment, so
the stored text carries the author's line breaks and the wrap has to come back out.

THE SIGNATURE BLOCK STRUCTURALLY REQUIRES IT: an unflattened continuation lands at column
0, where the block's own grammar reads it as an AUTHOR line - so a multi-line summary would
break the block, a case a single-line test fixture never exercises.

Normalizing HERE rather than at each consumer is what keeps the block, the standalone
signature and inspect's `summary` fields agreeing. `details` is prose and keeps its
newlines, which is why it stays on `read_doc`.

## fn with_summary
Shared by the block and the standalone form, so a summary is attached the same way wherever
a signature appears. The ` # ...` is omitted ENTIRELY when undocumented rather than left as
an empty comment.

## fn signature_of
THE TWO MODES OVER ONE IMPLEMENTATION. The TREE form passes the LEAF name, because the
indentation around it supplies the hierarchy; a STANDALONE help passes the FULL
NAMEPATH, because nothing around it does. Everything after that first token is identical,
which is the point - the two surfaces cannot drift.

The groups render from the index's typedefs, which cannot fail to render, so a line
needs no fallback.

## fn push_within_signatures
CALLS BEFORE SUBMODULES: the callables at a level are what a reader scans for, and a
submodule pushes the eye deeper. Each group sorts by name.

A module line is emitted when the module's OWN namepath is covered OR when anything below
it was. That second case is what keeps the ANCESTORS of a deep match present, so the
indentation still spells a whole namepath - a subtree without its ancestors cannot be turned
back into one, and the block's entire grammar is its indentation.

A PER-SIGNATURE PREDICATE rather than a rooted walk, because a purview is a SET of patterns:
two of them can root in different rigs at different depths, which no single root expresses,
while "does any of them cover this signature" composes for free. That predicate subsumes a
rooted walk - identical output for every single-pattern case - so one predicate covers it
rather than a predicate plus a separate rooted path.

The buffering is not stylistic: whether a module's own line belongs in the block is not
knowable until its subtree has been walked.

## fn render_signatures_within
THE ONE RENDERER. `interface info` shows the current purview through it, a pattern's
`interface help` shows one pattern through it, and the whole namespace is just the `*`
pattern - so none of the three can drift. This is the agent's FIRST read after the skill, so it is compact by
design: a structured nested-JSON tree of the same surface runs roughly 8 KB for just two
rigs.

NOTHING IS STATED THAT CAN BE INFERRED, and the separators are inferable from shape alone:
depth 0 is an author and depth 1 its equip's rig, the two levels of `<author>/<name>`;
everything deeper is a module UNLESS it carries the two signature groups, which makes it a
call. So a reader joins author to name with `/`, module segments with `/`, and puts `:`
before the first module and before the call.

A BUILTIN takes depth 0 alone, under its bare name, and leads the block: its line already
reads as the namepath an agent calls, `gram:log:latest`, with no author to join. Every
other rig follows under its author as before.

NO TRAILING HIERARCHY CHARACTERS, and the reasoning generalizes. A trailing separator would
let a line state its own separator rather than have one inferred, but it fails at exactly
the MIXED MODULE, which no single character describes: `a/b:c/` plus `:call` needs an
override rule, and an override rule IS re-inference wearing a costume. Shape settles every
case including that one, because a call is recognizable on its own.

Two further arguments favour SHAPE over separators. REUSE: a new kind extends the format by
taking a new SHAPE - a helper would be `name [...]` beside a call's `name <...> <...>` - rather
than by carving another character out of a two-character vocabulary that is already
overloaded. Separators do not scale; shapes do. And this block is the PROTOTYPE for a
general nu-tree renderer, which argues for keeping it general rather than shaped to
`interface info`.

A rig appears when its OWN namepath is covered or when anything inside it is, which is what
lets a bare `author/` list a rig with no calls yet while a pattern naming a module the rig
does not have leaves no orphan heading behind.

PATTERNS MATCHING NOTHING RENDER EMPTY rather than erroring. A pattern is a filter, and an
unresolved `.` matches nothing BY DESIGN, so an empty block is already this format's answer
for "nothing here" - a fresh namespace renders empty for the same reason.

A rig whose index fails to decode SKIPS with a host-stderr note rather than failing the
call, so one corrupt rig cannot take the surface down.

## fn rig_signatures
One rig's slice of the block, walked from the depth its rig line sits above: a builtin's
one level shallower than an author's rig. The rig's read lock is held only while its index
is read and walked.

## fn inspect_impl
Every path composes from the NAMEPATH against the COMMITTED canonical tree, so nothing here
reads `source_path`. Nothing agent-facing composes from it at all, so `source_path` is not
surfaced to the agent; it stays load-bearing INTERNALLY - `commit` re-reads it and
`check_source_dir` cross-checks it - so the index field stays.

A call's standalone signature names a builtin by its bare name, as the block does.
