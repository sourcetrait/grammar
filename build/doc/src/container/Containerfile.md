# Containerfile

The image a server's one-time build runs in, made by `run.bash` and removed
with the run.

## ARG BASE
The machine's own distribution, so what the container builds links against the
libraries the machine has: a binary built against a newer glibc than the
machine's refuses to start there. Fedora 44 is the default, the release's
first machines running it.

## RUN dnf install
A C toolchain and cmake for the dependencies that compile C, sqlite's,
libgit2's, zlib's, zstd's, and ring's among the workspace's; OpenSSL's headers
and pkg-config for a crate built against the system's TLS, which nushell's and
its plugins' own lockfiles, resolved at their install, may hold; git, tar, and
zstd; and rustup. Weak dependencies are left out and the package cache
cleaned, the image being thrown away anyway.

## RUN rustup-init
The pinned toolchain from rustup rather than the distribution's Rust, so every
machine builds with the same compiler whatever its distribution ships;
`minimal` carries rustc, cargo, and the standard library alone.
