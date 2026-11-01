# parse_engine.rs

## fn set_dirs_const
A parse-time CONST, not `$env`, and for the lib dirs that is the whole point.
nu-parser's `find_in_dirs` reads the const var FIRST, falling back to the
deprecated `$env.NU_LIB_DIRS` - so a body cannot override it, and the MCP's own
namespace dir, whose `equip/` holds every installed lib and rig, stays the sole
controlled lib path. The box's own `NU_LIB_DIRS` is excluded from the seeded env
for the same reason.

## struct ParseEngine

### fn new_full
Built through `startup`, exactly as the eval engines are, so rig source using
`$nu.*`, std, or `print` parses at commit as it does at serve. A const reading
`$nu.home-dir` is evaluated here at parse time, which is how an engine without
its config dirs failed a commit.

The `grimm` decls go in too, which each eval registers into its own working set: a
name the parser cannot resolve reads as an external, so without them a `grimm dbg`
handed a table literal failed an `execute` definition's contract parse and a rig's
validation alike, though the same text runs.

### fn engine_state
Borrows the base rather than cloning, because the lint's synthetic sources carry no
file references and therefore need no per-file PWD.

### fn engine_state_for_file
The `$env.PWD` it sets is what lets `export use ./<file>.nu` resolve a sibling flat
helper. The self-view goes FIRST in the dirs list so an equip's own equip-prefixed
references resolve against its source during its FIRST commit, before it has been
placed in the namespace.

## struct LintEngine
A `ParseEngine` snapshots the plugin decls at construction, exactly as the stateless
eval base does. A long-lived one therefore DRIFTS the moment a `plugin add` lands:
the eval side picks that change up through `Executor::refresh_base_if_stale`, so a
validator that never refreshed would reject rig source that RUNS - and reject it as
an opaque `ExtraPositional` mis-bind naming nothing about plugins.

This holder removes that asymmetry using the SAME registry-mtime signal the Executor
uses, applied to the other engine. A rebuild is heavier than the Executor's
clone-swap - a whole command context plus a registry read - which is why it fires
only on an actual mtime change rather than per call.

### fn current
ONE method rather than a separate refresh plus getter, so a caller cannot take the
engine and forget the refresh - which is precisely the bug this type exists to
remove. Poison-tolerant, like every other long-lived lock here.

## fn file_start
A span is an offset into the span space every file of a working set shares, each file's
text placed after the last; nushell's `parse` places a new file at `next_span_start()`.
Every position or slice the validator or the lint takes from a parse is measured from
here, which is what stays right when the engine holds files of its own, the standard
library's first.

## fn span_text
`None` for a span outside the file, so a span placed elsewhere reads as nothing rather
than as some other file's text.

## fn wrap_as_module
Returns its prefix length because every span the parse reports is against the
WRAPPED source, and a diagnostic has to point into the author's own file.
Subtracting the prefix is what translates one to the other.
