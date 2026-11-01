# dir.rs

## fn dir_tree
`**/.git` leads the ignore list, ahead of the caller's globs, so a regard
glob is the one way to render it.

## fn list
Globs match each entry's path relative to the tree's root with nu_glob's
default options, so a pattern names a path below the directory: `target`
matches the top-level `target` alone, `**/target` any, `target/**` the top
one's contents but not its own line, and a pattern with a trailing slash
matches nothing, since a relative path has none. A directory skipped is
never descended into, so a regard glob reaches no deeper than its ignored
ancestor.

## fn ignored
An ignore glob hides an entry and a regard glob rescues it; a regard glob
only rescues and never hides.
