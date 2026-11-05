# Build grammar's products and plugins, with nushell's own, into a signed OptZst.
#
# Usage: `nu build.nu [--debug]`, from anywhere; or in two halves on two
# machines, `nu build.nu [--debug] --tree <dir>` where the build runs and
# `nu build.nu --pack <dir>` where the signing key is.
#
# The package lands in cargo's target directory, in `pkg/`,
# `grammar_<semver>_<os>_<arch>.opt.tar.zst`; its path is the last line.
# `optzst` packs it, which must be on `PATH`, signing with the key whose file the
# repository's git config names as `optzst.key`. gram's repository must sit
# beside this one, as `gram`: the package carries its equip. A whole build
# tallies the dependencies' licenses into `LICENSES-EXTERNAL.txt` by
# cargo-license, `cargo license`, which must be installed.
#
# `--tree <dir>` builds and lays the package's tree out as `<dir>/package`, with
# `<dir>/install`, its own part, and `<dir>/build.nuon` naming its name, semver,
# os, and arch: no tally, no pack,
# no signature, so a machine without the key builds it, a server's tree among
# them. `--pack <dir>` packs such a `<dir>` into the signed OptZst, building
# nothing, so the key's machine runs no build. `--nu-sources <dir>` installs
# nushell and its core plugins offline from their published sources there,
# each `<crate>-<version>/`, as a build with no network needs.

const MOD_DIR = (path self | path expand | path dirname | path dirname | path join 'mod')
const BUNDLE = ($MOD_DIR | path join 'bundle')
use $BUNDLE

# The repository's root, from this script's real path.
const ROOT = (path self | path expand | path dirname | path dirname | path dirname)

# The package's own part: its record, whose name is its home's.
const INSTALL = ($ROOT | path join 'build' 'install')

# The record `--tree` writes beside its tree, which `--pack` reads.
const BUILD_RECORD = 'build.nuon'

def main [
    --debug        # cargo's dev profile; the package's semver takes `+dbg`
    --tree: path   # build and lay the tree out in this directory, no pack
    --pack: path   # pack the tree a `--tree` laid out in this directory
    --nu-sources: path # install nushell offline from the published sources here
]: nothing -> nothing {
    let profile: string = if $debug { 'debug' } else { 'release' }
    let name: string = (open --raw ($INSTALL | path join 'package.nuon') | from nuon | get name)
    if $pack != null {
        pack_tree ($pack | path expand) $name
        return
    }
    let key: oneof<nothing, path> = if $tree == null { signing_key } else { null }
    bundle require_gram $ROOT
    let target: record<os: string, arch: string> = (bundle target)
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
        let out: path = ($dir | path join 'package')
        if ($out | path exists) {
            rm --recursive $out
        }
        mkdir $dir
        cp --recursive $laid $out
        let own: path = ($dir | path join 'install')
        if ($own | path exists) {
            rm --recursive $own
        }
        cp --recursive $INSTALL $own
        {name: $name, semver: $semver, os: $target.os, arch: $target.arch} | to nuon | save --force ($dir | path join $BUILD_RECORD)
        print $out
        return
    }
    print 'packing and signing'
    let built: record<name: string, semver: string, os: string, arch: string> = {
        name: $name, semver: $semver, os: $target.os, arch: $target.arch
    }
    print (pack ($laid | path dirname) $built ((bundle target_dir $ROOT) | path join 'pkg') $key)
}

# The signing key's file from the repository's git config, refusing what could not pack.
def signing_key []: nothing -> path {
    if (which optzst | is-empty) {
        error make {msg: 'no optzst on PATH, which packs the package'}
    }
    let set: record<stdout: string, stderr: string, exit_code: int> = (
        ^git -C $ROOT config --get optzst.key | complete
    )
    if $set.exit_code != 0 {
        error make {msg: "set optzst.key in the repository's git config to the signing key's file"}
    }
    let key: path = ($set.stdout | str trim | path expand)
    if not ($key | path exists) {
        error make {msg: $"optzst.key names no file: ($key)"}
    }
    $key
}

# The tree a `--tree` laid out in `dir`, packed and signed beside it.
def pack_tree [dir: path, name: string]: nothing -> nothing {
    let built: record<name: string, semver: string, os: string, arch: string> = (
        open --raw ($dir | path join $BUILD_RECORD) | from nuon
    )
    if $built.name != $name {
        error make {msg: $"the tree is ($built.name)'s, the package record ($name)'s"}
    }
    print $"packing and signing ($built.name) ($built.semver) for ($built.os) ($built.arch)"
    print (pack $dir $built $dir (signing_key))
}

# `dir`'s `package/` and the package's own part packed by optzst; returns the package's path.
def pack [
    dir: path
    built: record<name: string, semver: string, os: string, arch: string>
    out: path
    key: path
]: nothing -> path {
    let own: path = ($dir | path join 'install')
    if ($own | path exists) {
        rm --recursive $own
    }
    cp --recursive $INSTALL $own
    let made: string = (
        ^optzst create $built.name $built.semver $built.os $built.arch $dir $out --key $key | lines | last
    )
    rm --recursive $own
    $made
}
