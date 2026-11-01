# Containerfile

The image a server's one-time build runs in, made by `run.bash` and removed
with the run.

## ARG BASE
The machine's own distribution, so what the container builds links against the
libraries the machine has: a binary built against a newer glibc than the
machine's refuses to start there. Fedora 44 is the default, the release's
first machines running it. A Debian-family base serves a machine Fedora
publishes no image for, Ubuntu's on riscv64.

## RUN
The base's package manager picks the family, `dnf` Fedora's and `apt-get`
Debian's, and a base with neither is refused. Each installs a C toolchain and
cmake for the dependencies that compile C, sqlite's, libgit2's, zlib's,
zstd's, and ring's among the workspace's; OpenSSL's headers and pkg-config for
a crate built against the system's TLS, which nushell's and its plugins' own
lockfiles, resolved at their install, may hold; git, tar, and zstd; Debian's
CA certificates, which its minimal image lacks and rustup's downloads need;
and rustup. Weak dependencies and recommendations are left out and the package
caches cleaned, the image being thrown away anyway.

The pinned toolchain comes from rustup rather than the distribution's Rust, so
every machine builds with the same compiler whatever its distribution ships;
`minimal` carries rustc, cargo, and the standard library alone. Fedora's rustup
package is `rustup-init`, which installs rustup with its proxies under
`/root/.cargo/bin`; Debian's is rustup itself, its proxies in `/usr/bin`, so it
installs the toolchain and makes it the default. Its default host is set from
the machine's architecture first, `riscv64` taking
`riscv64gc-unknown-linux-gnu`: Ubuntu's riscv64 rustup names its own host
`riscv64a23-unknown-linux-gnu`, for which Rust's channel publishes no
toolchain.
