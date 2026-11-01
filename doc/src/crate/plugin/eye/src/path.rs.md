# path.rs

## fn expand
A relative path resolves against the caller's directory, `~` expanded first:
nushell starts a plugin in its executable's directory. The caller's
directory is asked for only for a relative path, since an engine without
`$env.PWD` refuses the call.

## fn canonical
Canonicalized, so a missing path fails at the call naming itself, and a
tree's root line is the real path.
