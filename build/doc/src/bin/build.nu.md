# build.nu

The repository root's `build.nu` is a symlink to this script, so the script
never trusts the directory it was invoked from.

## The module imports
A relative `use` resolves against the directory of the path nushell was
handed, and through the root's symlink that is the root, where `../mod` does
not exist. `path self | path expand` follows the link to the real file, and
both it and `path dirname` evaluate at parse time, so the modules import
through constants built from it. Measured on 0.115.1 both ways: through the
link and directly.

## const ROOT
The same real path, three levels up, so the script builds the repository it
lives in from any working directory.

## const BUILD_RECORD
What `--pack` needs of the build, its name, semver, os, and arch, written
beside the tree, so the packing machine needs neither the build's cargo
metadata nor its rustc; `build/type/BuildRecord.nutype` is its model.

## def main
A whole build resolves the signer first, before any build, because a
repository without a usable key cannot produce a package and should say so in
a second rather than after the release builds. gram's equip is checked beside
it for the same reason. The licenses are tallied before the compile, from the
same manifest it builds.

`--tree` and `--pack` split the release at the key. The machine the tree is
for builds it, against its own libraries, with no key and no cargo-license,
which a whole build on the key's machine keeps current; the key's machine packs
and signs it, building nothing. A server's tree is built that way on the
server, in a one-time container (`build/container/run.bash`).

## def pack_tree
The tree is packed as a whole build packs its own, so a split release's
package is the same OptZip; it lands beside the tree, in the directory the
build left.
