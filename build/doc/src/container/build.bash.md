# build.bash

The container's side of a server's build: nushell first, then
`build.nu --tree`.

## The homes
Cargo's home and target directory sit in `/work`, the run's own directory on
the machine, mounted in: the registry's downloads and the build's artifacts go
there rather than into the container's layer, and `run.bash` removes the whole
directory when the run ends.

## nu_version
`build.nu` is a nushell script and the container holds no nushell, so the
first step builds one, at the version of `nu-protocol` in the workspace's
lock, the engine the MCP embeds. The lock is read with awk, no nushell existing
yet to read it. The install lands where `bundle nushell` installs, root and
target directory alike, so the bundle finds nushell installed at that version
and skips it, and the core plugins' builds reuse what nushell's compiled.

## build.nu --tree
The tree alone: no license tally, which needs cargo-license, and no signature,
whose key never reaches a server; the key's machine packs and signs the tree
(`build.nu --pack`).
