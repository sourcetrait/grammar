# grammar.spec

The rpm's spec, its `@VERSION@` and `@NU_VERSION@` filled in by `srpm.bash`.

## The package
One package of the whole tree, `/opt/grammar` OptGrammar's layout, nothing
outside it required: the binaries, nushell, its plugins, the equips, and the
skills, as the OptZst carries them. `/usr/bin` links each entry of the tree's
`bin/`, so both products are on the path and nushell's `nu`, under `nu/bin/`,
shadows no other. No debuginfo package is made. The architectures are the
release's Linux targets.

## %build
cargo runs offline against the vendored crates, its home the source's own
`cargo-home`, whose configuration points crates.io at `vendor/` once `%build`
fills in its path. nushell is built first, `build.nu` being a nushell script,
at the root and target directory `bundle nushell` uses, so the bundle finds it
installed and installs the core plugins alone, from the published sources by
`--nu-sources`. `build.nu --tree` then builds and lays out the tree as every
target's build does.

## %files
The licenses from grammar's root, AGPL-3.0's text and the dependencies'
tally, then the tree and the two links.
