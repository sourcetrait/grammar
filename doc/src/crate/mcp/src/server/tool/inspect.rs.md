# inspect.rs

## struct InspectResult
EXACTLY ONE FIELD. The namepath is not reprinted because the caller supplied it, and the
per-kind shapes carry nothing in common worth hoisting beside it.

## fn help
The two words come before any namepath: `interface` answers the interface's signature
block, its deny set applied, and `grimm` the grimm commands' (User). Both are held from
equip names (`engine:equip/validate.rs`), so no namepath they could shadow exists.

CLASSIFICATION IS BY SHAPE AND HAPPENS FIRST. A trailing hierarchy character, or a bare `*`
or `.`, makes this a PATTERN; anything else validates as the exact namepath it always did,
with the exact parser's strictness unchanged.

`.` IS RESOLVED HERE, not in the parser, and that placement is the design: purview is the
only thing that knows what `.` names, so the parser leaves it unresolved and an unresolved
`.` matches nothing. Resolving it at this one call site is what makes `interface help .`
render the current view rather than an empty block.

An EXACT namepath is the engine's `Help`, read under its one equip's lock. A PATTERN SPANS
EQUIPS, so it renders as a signature block through the engine's `Signatures`, whose
renderer takes each equip's lock itself.

The two words answer the MCP's own blocks whatever the host, the interface being the
MCP's and grimm the same on every engine; a namepath or a pattern goes to the routed
engine, `.` resolving against the session's view there.
