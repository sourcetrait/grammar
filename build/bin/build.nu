# Build grammar's products and plugins, with nushell's own, into a signed OptZst.
#
# Usage: `nu build.nu [--debug]`, from anywhere; or in two halves on two
# machines, `nu build.nu [--debug] --tree <dir>` where the build runs and
# `nu build.nu --pack <dir>` where the signing key is.
#
# The package lands in cargo's target directory, in `pkg/`,
# `grammar_<semver>_<os>_<arch>.opt.tar.zst`; its path is the last line.
# It is signed by the repository's gpg commit key, or by `gpg.signingkey`, a
# full fingerprint, with `gpg.homedir` the GnuPG home. gram's repository must
# sit beside this one, as `gram`: the package carries its equip. A whole build
# tallies the dependencies' licenses into `LICENSES-EXTERNAL.txt` by
# cargo-license, `cargo license`, which must be installed.
#
# `--tree <dir>` builds and lays the package's tree out as `<dir>/tree`, with
# `<dir>/build.nuon` naming its name, semver, os, and arch: no tally, no pack,
# no signature, so a machine without the key builds it, a server's tree among
# them. `--pack <dir>` packs such a `<dir>` into the signed OptZst, building
# nothing, so the key's machine runs no build. `--nu-sources <dir>` installs
# nushell and its core plugins offline from their published sources there,
# each `<crate>-<version>/`, as a build with no network needs.

const MOD_DIR = (path self | path expand | path dirname | path dirname | path join 'mod')
const BUNDLE = ($MOD_DIR | path join 'bundle')
const OPTZST = ($MOD_DIR | path join 'optzst')
use $BUNDLE
use $OPTZST

# The repository's root, from this script's real path.
const ROOT = (path self | path expand | path dirname | path dirname | path dirname)

# The package's record, its name that of its home, `opt/grammar`.
const PACKAGE = ($ROOT | path join 'build' 'package.nuon')

# The record `--tree` writes beside its tree, which `--pack` reads.
const BUILD_RECORD = 'build.nuon'

def main [
    --debug        # cargo's dev profile; the package's semver takes `+dbg`
    --tree: path   # build and lay the tree out in this directory, no pack
    --pack: path   # pack the tree a `--tree` laid out in this directory
    --nu-sources: path # install nushell offline from the published sources here
]: nothing -> nothing {
    let profile: string = if $debug { 'debug' } else { 'release' }
    let name: string = (optzst package_record $PACKAGE | get name)
    if $pack != null {
        pack_tree ($pack | path expand) $name
        return
    }
    let signer: oneof<nothing, record<key: string, home: oneof<nothing, string>>> = (
        if $tree == null { optzst signer $ROOT } else { null }
    )
    bundle require_gram $ROOT
    let target: record<os: string, arch: string> = (optzst target)
    let semver: string = (bundle package_semver $ROOT $profile)
    print $"building ($name) ($semver) for ($target.os) ($target.arch)"
    if $tree == null {
        print "tallying the dependencies' licenses"
        bundle licenses $ROOT
    }
    bundle compile $ROOT $profile
    print 'installing nushell and its core plugins'
    let nu_root: path = (bundle nushell $ROOT $profile $nu_sources)
    print 'laying out the package'
    let laid: path = (bundle layout $ROOT $profile $nu_root $target.os)
    if $tree != null {
        let dir: path = ($tree | path expand)
        let out: path = ($dir | path join 'tree')
        if ($out | path exists) {
            rm --recursive $out
        }
        mkdir $dir
        cp --recursive $laid $out
        {name: $name, semver: $semver, os: $target.os, arch: $target.arch} | to nuon | save --force ($dir | path join $BUILD_RECORD)
        print $out
        return
    }
    let file: string = (optzst file_name $name $semver $target.os $target.arch)
    print 'packing and signing'
    let out: path = (optzst pack $laid $PACKAGE ((bundle target_dir $ROOT) | path join 'pkg' $file) $signer)
    print $out
}

# The tree a `--tree` laid out in `dir`, packed and signed beside it.
def pack_tree [dir: path, name: string]: nothing -> nothing {
    let built: record<name: string, semver: string, os: string, arch: string> = (
        open --raw ($dir | path join $BUILD_RECORD) | from nuon
    )
    if $built.name != $name {
        error make {msg: $"the tree is ($built.name)'s, the package record ($name)'s"}
    }
    let signer: record<key: string, home: oneof<nothing, string>> = (optzst signer $ROOT)
    let file: string = (optzst file_name $built.name $built.semver $built.os $built.arch)
    print $"packing and signing ($file)"
    let out: path = (optzst pack ($dir | path join 'tree') $PACKAGE ($dir | path join $file) $signer)
    print $out
}
