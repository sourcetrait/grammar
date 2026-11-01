# srpm.bash

The source rpm's assembly, the one step of the rpm's build with the network.

## The version
The MCP crate's version is the release's, read by `cargo pkgid`, and held to
a semver before anything uses it. rpm sorts `~` before anything, so the draft
counter after a tilde, `0.0.0~10` for 0.0.0-10, sorts a release after its
drafts. The engine's nushell version is `nu-protocol`'s in grammar's lock,
read with awk, the container holding no nushell.

## The sources
Each repository is copied by tar without its `target/` or `.git`, so a
checkout mounted as it stands adds no build output or history to the source.
nushell and its core plugins come from crates.io as published at the engine's
version, as `bundle nushell` installs them, unpacked as `<crate>-<version>/`
for the spec's `--nu-sources`.

## The vendor directory
One `cargo vendor` fills one directory for grammar's workspace and, by
`--sync`, every nushell manifest, each resolved by its own lock, `--locked`,
`--versioned-dirs` keeping two versions of a crate apart. The configuration it
prints replaces crates.io with that directory; its absolute path is this
container's, so `@VENDOR@` stands in for it and the spec's `%build` fills in
the builder's own.

## The source rpm
`rpmbuild -bs` packs the spec, its version and nushell's filled in, with the
source as one zstd tarball, into `/out`.
