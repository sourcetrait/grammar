# tree.rs

The compact tree both commands render: a line a directory or a file, a space
of indent a level, a directory's files and symlinks before its directories,
each group sorted by its names' raw bytes, case-sensitive.

## fn render
`list` gives each directory's entries, so `eye dir`'s globs and `eye git`'s
ignore rules share one renderer, and a tree reads the same from either when
nothing is ignored. Every line ends in a newline, the last included: a tree
without its final newline was cut.

## fn size_label
1024-based with an integer floor, its units lowercase, and `0` bare for an
empty file: a glance, not a byte count.

## fn wrap
A name holding a space is backtick-wrapped, so the name reads as one token
before its size.

## fn symlink_label
A symlink is a leaf, never followed: its canonical target, a directory's with
a trailing `/` and a file's with its size; a broken link shows its raw
target.
