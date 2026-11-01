# Build grammar-mcp, its worker, and its plugins, with nushell's own, into a signed OptZip.
#
# Usage: `nu build.nu [--debug]`, from anywhere; or in two halves on two
# machines, `nu build.nu [--debug] --tree <dir>` where the build runs and
# `nu build.nu --pack <dir>` where the signing key is.
#
# The package lands in cargo's target directory, in `pkg/`,
# `grammar-mcp_<semver>_<os>_<arch>.opt.tar.zst`; its path is the last line.
# It is signed by the repository's gpg commit key, or by `gpg.signingkey`, a
# full fingerprint, with `gpg.homedir` the GnuPG home. gram's repository must
# sit beside this one, as `gram`: the package carries its equip. A whole build
# tallies the dependencies' licenses into `LICENSES-EXTERNAL.txt` by
# cargo-license, `cargo license`, which must be installed.
#
# `--tree <dir>` builds and lays the package's tree out as `<dir>/tree`, with
# `<dir>/build.nuon` naming its name, semver, os, and arch: no tally, no pack,
# no signature, so a machine without the key builds it, a server's tree among
# them. `--pack <dir>` packs such a `<dir>` into the signed OptZip, building
# nothing, so the key's machine runs no build.

const MOD_DIR = (path self | path expand | path dirname | path dirname | path join 'mod')
const BUNDLE = ($MOD_DIR | path join 'bundle')
const OPTZIP = ($MOD_DIR | path join 'optzip')
use $BUNDLE
use $OPTZIP

# The repository's root, from this script's real path.
const ROOT = (path self | path expand | path dirname | path dirname | path dirname)

# The program the package carries.
const NAME = 'grammar-mcp'

# The record `--tree` writes beside its tree, which `--pack` reads.
const BUILD_RECORD = 'build.nuon'

def main [
    --debug        # cargo's dev profile; the package's semver takes `+dbg`
    --tree: path   # build and lay the tree out in this directory, no pack
    --pack: path   # pack the tree a `--tree` laid out in this directory
]: nothing -> nothing {
    let profile: string = if $debug { 'debug' } else { 'release' }
    if $pack != null {
        pack_tree ($pack | path expand)
        return
    }
    let signer: oneof<nothing, record<key: string, home: oneof<nothing, string>>> = (
        if $tree == null { optzip signer $ROOT } else { null }
    )
    bundle require_gram $ROOT
    let target: record<os: string, arch: string> = (optzip target)
    let semver: string = (bundle package_semver $ROOT $profile)
    print $"building ($NAME) ($semver) for ($target.os) ($target.arch)"
    if $tree == null {
        print "tallying the dependencies' licenses"
        bundle licenses $ROOT
    }
    bundle compile $ROOT $profile
    print 'installing nushell and its core plugins'
    let nu_root: path = (bundle nushell $ROOT $profile)
    print 'laying out the package'
    let laid: path = (bundle layout $ROOT $profile $nu_root)
    if $tree != null {
        let dir: path = ($tree | path expand)
        let out: path = ($dir | path join 'tree')
        if ($out | path exists) {
            rm --recursive $out
        }
        mkdir $dir
        cp --recursive $laid $out
        {name: $NAME, semver: $semver, os: $target.os, arch: $target.arch} | to nuon | save --force ($dir | path join $BUILD_RECORD)
        print $out
        return
    }
    let file: string = (optzip file_name $NAME $semver $target.os $target.arch)
    print 'packing and signing'
    let out: path = (optzip pack $laid $NAME ((bundle target_dir $ROOT) | path join 'pkg' $file) $signer)
    print $out
}

# The tree a `--tree` laid out in `dir`, packed and signed beside it.
def pack_tree [dir: path]: nothing -> nothing {
    let built: record<name: string, semver: string, os: string, arch: string> = (
        open --raw ($dir | path join $BUILD_RECORD) | from nuon
    )
    let signer: record<key: string, home: oneof<nothing, string>> = (optzip signer $ROOT)
    let file: string = (optzip file_name $built.name $built.semver $built.os $built.arch)
    print $"packing and signing ($file)"
    let out: path = (optzip pack ($dir | path join 'tree') $built.name ($dir | path join $file) $signer)
    print $out
}
