# The OptZst package: a program's signed tars for an opt home.
#
# `<name>_<semver>_<os>_<arch>.opt.tar.zst`, no field holding `_`: a zstd tar
# of exactly six files, `<name>.tar` and `package-<name>.tar`, each with its
# sha256sum record and its detached OpenPGP signature. The package tar holds
# `package.nuon` and the install scripts, `install.bash` and `install.ps1`,
# which unpack the inner tar into an opt home.

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

# The install scripts every package carries, kept beside this module.
const SCRIPT_DIR = (path self | path dirname)
const SCRIPTS = ['install.bash' 'install.ps1']

# The fields a `package.nuon` holds, sorted.
const PACKAGE_FIELDS = ['name' 'summary' 'title']

# The longest summary a `package.nuon` may hold.
const SUMMARY_MAX = 80

# The package file's name, refusing a field that is empty or holds `_`.
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
            error make {msg: $"an OptZst ($row.field) is empty or holds `_`: `($row.value)`"}
        }
    }
    $"($name)_($semver)_($os)_($arch).opt.tar.zst"
}

# A package's `package.nuon`: its name, title, and summary, each checked.
export def package_record [
    package: path
]: nothing -> record<name: string, title: string, summary: string> {
    let read: record = (open --raw $package | from nuon)
    if ($read | columns | sort) != $PACKAGE_FIELDS {
        error make {msg: $"($package) holds `name`, `title`, and `summary` alone"}
    }
    for field in $PACKAGE_FIELDS {
        let value: any = ($read | get $field)
        if ($value | describe) != 'string' or ($value | str trim | is-empty) {
            error make {msg: $"($package): `($field)` is no non-empty string"}
        }
    }
    if ($read.summary | str length) > $SUMMARY_MAX {
        error make {msg: $"($package): the summary runs past ($SUMMARY_MAX) characters"}
    }
    {name: $read.name, title: $read.title, summary: $read.summary}
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

# Pack a tree and its package's record as the OptZst `out`, signed and verified; returns `out`.
#
# The archive is built beside `out` and replaces it only once both signatures
# verify, so a failure leaves an earlier package intact.
export def pack [
    tree: path      # the package's contents, its home's top level
    package: path   # the package's `package.nuon`
    out: path       # the package file to write
    signer: record<key: string, home: oneof<nothing, string>>
]: nothing -> path {
    let name: string = (package_record $package | get name)
    let stage: path = ($out | path dirname | path join $".($name)-stage")
    if ($stage | path exists) {
        rm --recursive $stage
    }
    mkdir $stage
    let inner: string = $"($name).tar"
    let members: list<string> = (ls $tree | get name | each {|p| $p | path basename } | sort)
    ^tar --create --file ($stage | path join $inner) --directory $tree ...$REPRODUCIBLE ...$members
    let installer: string = $"package-($name).tar"
    let contents: path = ($stage | path join '.package')
    mkdir $contents
    cp $package ($contents | path join 'package.nuon')
    for script in $SCRIPTS {
        cp ($SCRIPT_DIR | path join $script) ($contents | path join $script)
    }
    ^chmod 0644 ($contents | path join 'package.nuon')
    ^chmod 0755 ...($SCRIPTS | each {|script| $contents | path join $script })
    (^tar --create --file ($stage | path join $installer) --directory $contents
        ...$REPRODUCIBLE 'package.nuon' ...$SCRIPTS)
    rm --recursive $contents
    let keyring: path = ($stage | path join 'signer.gpg')
    let gnupg: record = if ($signer.home == null) { {} } else { {GNUPGHOME: $signer.home} }
    let files: list<string> = ([$inner $installer] | each {|tar|
        let digest: string = (open --raw ($stage | path join $tar) | hash sha256)
        $"($digest)  ($tar)\n" | save ($stage | path join $"($tar).sha256")
        with-env $gnupg {
            (^gpg --batch --yes --local-user $signer.key --digest-algo SHA256
                --output ($stage | path join $"($tar).gpg.sig") --detach-sign ($stage | path join $tar))
        }
        [$tar $"($tar).sha256" $"($tar).gpg.sig"]
    } | flatten)
    with-env $gnupg {
        ^gpg --batch --yes --output $keyring --export $signer.key
    }
    for tar in [$inner $installer] {
        ^gpgv --keyring $keyring ($stage | path join $"($tar).gpg.sig") ($stage | path join $tar)
    }
    let partial: path = ($out | path dirname | path join $".($out | path basename).partial")
    (^tar --create --file $partial --use-compress-program 'zstd -q -T1 -10'
        --directory $stage ...$REPRODUCIBLE ...$files)
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
