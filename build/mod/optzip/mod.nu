# The OptZip package: a program's signed tar for an opt home.
#
# `<name>_<semver>_<os>_<arch>.opt.tar.zst`, no field holding `_`: a zstd tar
# of exactly `<name>.tar`, its sha256sum record, and its detached OpenPGP
# signature. It deploys to `$UENV_USR_LOCAL/opt/<name>`.

# The tar options that make an archive's bytes depend on its content alone.
const REPRODUCIBLE = [
    '--sort=name'
    '--format=posix'
    '--pax-option=exthdr.name=%d/PaxHeaders/%f,delete=atime,delete=ctime'
    '--mtime=@0'
    '--owner=0'
    '--group=0'
    '--numeric-owner'
]

# The package's file name, refusing a field that is empty or holds `_`.
export def file_name [
    name: string
    semver: string
    os: string
    arch: string
]: nothing -> string {
    let fields: table<field: string, value: string> = [
        [field value];
        [name $name] [semver $semver] [os $os] [arch $arch]
    ]
    for row in $fields {
        if ($row.value | is-empty) or ($row.value | str contains '_') {
            error make {msg: $"an OptZip ($row.field) is empty or holds `_`: `($row.value)`"}
        }
    }
    $"($name)_($semver)_($os)_($arch).opt.tar.zst"
}

# The build target's os and arch: Rust's names, `_` written `-`.
export def target []: nothing -> record<os: string, arch: string> {
    let cfg: table<key: string, value: string> = (
        ^rustc --print cfg | lines | parse '{key}="{value}"'
    )
    let pick = {|key: string|
        $cfg | where key == $key | get value.0 | str replace --all '_' '-'
    }
    {os: (do $pick 'target_os'), arch: (do $pick 'target_arch')}
}

# The signing key from a repository's git config, and its GnuPG home.
#
# A repository signing commits with gpg signs with that key; any other needs
# `gpg.signingkey`, a full fingerprint. `GNUPGHOME`, else `gpg.homedir`, is
# the GnuPG home.
export def signer [
    repo: path
]: nothing -> record<key: string, home: oneof<nothing, string>> {
    let format: string = (git_config $repo 'gpg.format' | default 'openpgp')
    let commit_key: oneof<nothing, string> = (git_config $repo 'user.signingkey')
    let key: string = if ($format == 'openpgp') and ($commit_key != null) {
        $commit_key
    } else {
        let configured: oneof<nothing, string> = (git_config $repo 'gpg.signingkey')
        if $configured == null {
            error make {
                msg: ('the repository does not sign commits with gpg: set '
                    + 'gpg.signingkey to the full OpenPGP fingerprint')
            }
        }
        let fingerprint: string = ($configured | str replace --all ' ' '' | str uppercase)
        if not ($fingerprint =~ '^([0-9A-F]{40}|[0-9A-F]{64})$') {
            error make {msg: $"gpg.signingkey is no full OpenPGP fingerprint: `($configured)`"}
        }
        $fingerprint
    }
    let home_setting: oneof<nothing, string> = (git_config $repo 'gpg.homedir')
    let home: oneof<nothing, string> = if ($env.GNUPGHOME? != null) {
        $env.GNUPGHOME
    } else if ($home_setting != null) {
        $home_setting | path expand
    } else {
        null
    }
    {key: $key, home: $home}
}

# Pack a tree as the OptZip `out`, signed and verified; returns `out`.
#
# The archive is built beside `out` and replaces it only once its signature
# verifies, so a failure leaves an earlier package intact.
export def pack [
    tree: path    # the package's contents, its home's top level
    name: string  # the program's name, the inner tar's stem
    out: path     # the package file to write
    signer: record<key: string, home: oneof<nothing, string>>
]: nothing -> path {
    let stage: path = ($out | path dirname | path join $".($name)-stage")
    if ($stage | path exists) {
        rm --recursive $stage
    }
    mkdir $stage
    let inner: string = $"($name).tar"
    let members: list<string> = (ls $tree | get name | each {|p| $p | path basename } | sort)
    ^tar --create --file ($stage | path join $inner) --directory $tree ...$REPRODUCIBLE ...$members
    let record: string = $"($inner).sha256"
    let digest: string = (open --raw ($stage | path join $inner) | hash sha256)
    $"($digest)  ($inner)\n" | save ($stage | path join $record)
    let signature: string = $"($inner).gpg.sig"
    let keyring: path = ($stage | path join 'signer.gpg')
    let gnupg: record = if ($signer.home == null) { {} } else { {GNUPGHOME: $signer.home} }
    with-env $gnupg {
        (^gpg --batch --yes --local-user $signer.key --digest-algo SHA256
            --output ($stage | path join $signature) --detach-sign ($stage | path join $inner))
        ^gpg --batch --yes --output $keyring --export $signer.key
    }
    ^gpgv --keyring $keyring ($stage | path join $signature) ($stage | path join $inner)
    let partial: path = ($out | path dirname | path join $".($out | path basename).partial")
    (^tar --create --file $partial --use-compress-program 'zstd -q -T1 -10'
        --directory $stage ...$REPRODUCIBLE $inner $record $signature)
    mv --force $partial $out
    rm --recursive $stage
    $out
}

# One git config value in a repository, or nothing when it is unset.
def git_config [repo: path, key: string]: nothing -> oneof<nothing, string> {
    let result: record<stdout: string, stderr: string, exit_code: int> = (
        ^git -C $repo config --get $key | complete
    )
    if $result.exit_code == 0 { $result.stdout | str trim } else { null }
}
