# git.rs

## fn git_tree
The ignore crate's walk, ripgrep's, applies git's rules: every `.gitignore`
below the root, and above it up to the repository's root, with a deeper
file's pattern, a negation among them, winning over a shallower one's. Its
`require_git` stays on, git's own scope: outside a repository no `.gitignore`
applies, and none above the repository's root is read. `.git/info/exclude`,
the global excludes file, `.ignore` files, and the hidden filter are off, so
the command's rules are the `.gitignore` files alone, with `.git` skipped by
name at any depth.

The walk's entries group by their parent and the shared renderer sorts each
group, so the walk's own order only keeps it deterministic. Symlinks are not
followed.
