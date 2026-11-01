# soak_dir.rs

## fn resolve_arg
A relative `src` or `dst` resolves against the caller's directory, `~`
expanded first. nushell starts a plugin in its executable's directory and
expands only a directory literal's tilde, so a relative path taken as the
plugin received it resolved against the plugin's own directory.

The caller's directory is asked for only for a relative path: an absolute one
needs no engine call, and an engine without `$env.PWD` refuses the call.
