# render.rs

`soak dir` and the two string commands render here.

## fn render_dir
THREE PHASES, AND NOTHING WRITTEN UNTIL THE FIRST TWO PASS. The plan walks the
source and maps each entry to its output; the stage parses and renders every
template into memory; the write makes the directories and writes the
outputs. A missing source, a partial or template that fails to parse or
render, a fill Liquid cannot hold, a collision, or an overlap fails the soak
before the destination exists. What remains is an I/O failure while writing,
which can leave the outputs written before it: a soak renders in place,
keeping every other file in the destination, so there is no whole tree to
swap in.

The source is canonicalized and the destination resolved through its longest
existing ancestor, so the overlap checks compare real paths. Both go through
dunce, which answers the plain path on Windows where std answers the verbatim
`\\?\` form, so the two compare alike and an error names the path as written. A destination at
or below the source would walk its own output on the next run, and an output
below the source would overwrite a template.

## fn plan
Entries come in name order, so a run's outputs and its errors are the same
each time. Symlinks are followed: a linked template renders and a linked
directory is walked, and walkdir refuses a loop. An entry that is neither a
file nor a directory, a fifo or a socket, is refused rather than copied,
since copying a fifo blocks.

A directory and a file claim their output path alike, so `a.txt/` beside
`a.txt.liquid` is a collision, found before anything is written.

## fn walk
No entry or depth cap: the walk sees the whole tree. dir_walker, which it
replaced, stopped silently past 10,000 entries and at depth 100, and it
unwrapped the canonicalized path of each directory it was told to skip, so a
source without `liquid/` crashed the plugin.

## fn undotted
Dottedness is an entry's own name below the walk's root. dir_walker tested
for `/.` anywhere in the path, so a source under any dotted directory
rendered nothing.

## fn relative
The components past the root's own, so no prefix match can fail: walkdir
builds every path from the root it was given.

## fn resolve
A destination that does not exist yet still resolves: its longest existing
ancestor is canonicalized and the missing names are kept, so a symlink above
it counts in the overlap checks.

## fn build_memory_partials
A partial is a `.liquid` or extensionless file below `liquid/`, named by its
path there with `.liquid` trimmed, as `include` and `render` name it:
`liquid/thought.liquid` is `thought`, `liquid/impartial.md.liquid` is
`impartial.md`. Dotted names are skipped there as in the source.

## fn value_to_liquid_value
nushell's scalars map to Liquid's; a filesize, range, or cell path as its
string; a duration in humantime's form; a date through Liquid's date parser;
a list to an array, a record to an object. A custom value converts by its base
value, and a base value that is itself custom is refused, a guard beyond
nushell's contract for `to_base_value`. A closure or an error is refused.
