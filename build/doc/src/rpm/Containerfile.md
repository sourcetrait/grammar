# Containerfile

The image the rpm's one-time build runs in, made by `run.bash` and removed
with the run.

## ARG BASE
The Fedora release the rpm is for, the release's builder in copr being that
release's own; Fedora 44 is the default.

## RUN
Fedora's own Rust and cargo rather than rustup's, since copr builds a source
rpm with the distribution's toolchain, what the spec's `BuildRequires` name:
the rebuild here proves the build copr runs. The C toolchain, cmake, OpenSSL's
headers, and pkg-config serve the crates that compile C, as the server build's
image does; curl fetches nushell's published crates; `rpm-build` provides
`rpmbuild`; tar and zstd pack the source. Weak dependencies are left out and
the package cache cleaned, the image being thrown away anyway.
