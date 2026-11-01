# The OptZst install scripts held to their cases, under one shell.
#
# Usage: `nu install.nu <bash|pwsh>`, from anywhere. A fixture package, its
# inner tar, record, and `package.nuon` beside the two scripts, is made in a
# scratch directory; each case runs the script there and is checked by its
# exit, its last line, and the files and links it leaves. The table prints,
# and any failed case exits 1. A Windows run needs an elevated session for its
# links.

const SCRIPTS = (path self | path expand | path dirname | path dirname | path join 'mod' 'optzst')

# The fixture's name, a valid OptZst name.
const NAME = 'fixture'

def main [shell: string]: nothing -> nothing {
    if $shell not-in ['bash' 'pwsh'] {
        error make {msg: $"no shell `($shell)`: bash or pwsh"}
    }
    let scratch: path = (mktemp --directory --tmpdir 'optzst-install.XXXXXX')
    let package: path = ($scratch | path join 'package')
    fixture $package
    let unix: bool = ($nu.os-info.name != 'windows')
    let results: list<record<case: string, ok: bool, detail: string>> = [
        (case_fresh $shell $package $scratch)
        (case_reinstall $shell $package $scratch)
        (case_dir_needs_dir $shell $package)
        (case_dir_belongs_to_dir $shell $package $scratch)
        (case_uenv_unset $shell $package)
        (case_uenv_links $shell $package $scratch $unix)
        (case_bin_file $shell $package $scratch)
        (case_foreign_home $shell $package $scratch)
        (case_tampered $shell $package $scratch)
        (case_unknown_place $shell $package)
        (case_no_place $shell $package)
        (case_unknown_argument $shell $package)
    ]
    rm --recursive $scratch
    print ($results | table --expand)
    if ($results | any {|r| not $r.ok }) {
        exit 1
    }
}

# A package's members as the install sees them, beside its two scripts.
def fixture [dir: path]: nothing -> nothing {
    let tree: path = ($dir | path join 'tree')
    mkdir ($tree | path join 'bin') ($tree | path join 'share')
    "#!/bin/sh\necho alpha\n" | save ($tree | path join 'bin' 'alpha')
    "#!/bin/sh\necho beta\n" | save ($tree | path join 'bin' 'beta')
    "fixture\n" | save ($tree | path join 'share' 'readme.txt')
    if $nu.os-info.name != 'windows' {
        ^chmod 0755 ($tree | path join 'bin' 'alpha') ($tree | path join 'bin' 'beta')
    }
    let tarball: path = ($dir | path join $"($NAME).tar")
    ^tar --create --file $tarball --directory $tree bin share
    rm --recursive $tree
    $"(open --raw $tarball | hash sha256)  ($NAME).tar\n" | save $"($tarball).sha256"
    {name: $NAME, title: Fixture, summary: 'The OptZst install scripts test package'} | to nuon | save ($dir | path join 'package.nuon')
    for script in ['install.bash' 'install.ps1'] {
        cp ($SCRIPTS | path join $script) ($dir | path join $script)
    }
}

# The package copied into `dir`, for a case that changes it.
def copy_package [package: path, dir: path]: nothing -> path {
    mkdir $dir
    ls $package | get name | each {|f| cp $f $dir } | ignore
    $dir
}

# One run of the shell's script in `package`, with `vars` set; an empty value reads as unset.
def invoke [
    shell: string
    package: path
    args: list<string>
    vars: record
]: nothing -> record<exit: int, last: string, err: string> {
    let result: record<stdout: string, stderr: string, exit_code: int> = (with-env $vars {
        if $shell == 'bash' {
            ^bash ($package | path join 'install.bash') ...$args | complete
        } else {
            ^pwsh -NoProfile -NonInteractive -File ($package | path join 'install.ps1') ...$args | complete
        }
    })
    {
        exit: $result.exit_code
        last: ($result.stdout | str trim | lines | last 1 | str join '')
        err: ($result.stderr | str trim)
    }
}

# A refusal: exit 1, its message holding `expect`.
def refused [name: string, result: record<exit: int, last: string, err: string>, expect: string]: nothing -> record<case: string, ok: bool, detail: string> {
    let ok: bool = ($result.exit == 1) and ($result.err | str contains $expect)
    {case: $name, ok: $ok, detail: (if $ok { '' } else { $"exit ($result.exit): ($result.err | str substring 0..200)" })}
}

# Each check holding: the case, and the first that fails.
def checked [name: string, checks: list<record<what: string, held: bool>>]: nothing -> record<case: string, ok: bool, detail: string> {
    let failed: list<string> = ($checks | where held == false | get what)
    {case: $name, ok: ($failed | is-empty), detail: ($failed | str join '; ')}
}

def case_fresh [shell: string, package: path, scratch: path]: nothing -> record<case: string, ok: bool, detail: string> {
    let home: path = ($scratch | path join 'fresh')
    let result = (invoke $shell $package ['--place' 'dir' '--dir' $home] {})
    checked 'a fresh dir install' [
        {what: $"exit ($result.exit): ($result.err)", held: ($result.exit == 0)}
        {what: 'the last line is the home', held: (($result.last | path expand) == ($home | path expand))}
        {what: 'bin/alpha unpacked', held: (($home | path join 'bin' 'alpha') | path exists)}
        {what: 'share/readme.txt unpacked', held: (($home | path join 'share' 'readme.txt') | path exists)}
        {what: 'package.nuon marks the home', held: (($home | path join 'package.nuon') | path exists)}
    ]
}

def case_reinstall [shell: string, package: path, scratch: path]: nothing -> record<case: string, ok: bool, detail: string> {
    let home: path = ($scratch | path join 'fresh')
    "stale\n" | save --force ($home | path join 'share' 'stale.txt')
    let result = (invoke $shell $package ['--place' 'dir' '--dir' $home] {})
    checked 'an earlier install emptied and replaced' [
        {what: $"exit ($result.exit): ($result.err)", held: ($result.exit == 0)}
        {what: 'a file the package lacks is gone', held: (not (($home | path join 'share' 'stale.txt') | path exists))}
        {what: 'bin/alpha unpacked', held: (($home | path join 'bin' 'alpha') | path exists)}
    ]
}

def case_dir_needs_dir [shell: string, package: path]: nothing -> record<case: string, ok: bool, detail: string> {
    refused '--place dir without --dir' (invoke $shell $package ['--place' 'dir'] {}) 'needs --dir'
}

def case_dir_belongs_to_dir [shell: string, package: path, scratch: path]: nothing -> record<case: string, ok: bool, detail: string> {
    let result = (invoke $shell $package ['--place' 'uenv-usr-adhoc' '--dir' ($scratch | path join 'x')] {UENV_USR_ADHOC: ($scratch | path join 'adhoc-x')})
    refused '--dir with a uenv place' $result 'belongs to --place dir alone'
}

def case_uenv_unset [shell: string, package: path]: nothing -> record<case: string, ok: bool, detail: string> {
    refused 'a uenv place unset' (invoke $shell $package ['--place' 'uenv-usr-adhoc'] {UENV_USR_ADHOC: ''}) 'UENV_USR_ADHOC is not set'
}

def case_uenv_links [shell: string, package: path, scratch: path, unix: bool]: nothing -> record<case: string, ok: bool, detail: string> {
    let base: path = ($scratch | path join 'adhoc')
    let bin: path = ($base | path join 'bin')
    mkdir $bin
    let stale: string = (['..' 'opt' $NAME 'bin' 'gamma'] | path join)
    if $unix {
        ^ln -s $stale ($bin | path join 'gamma')
        ^ln -s '/bin/true' ($bin | path join 'other')
    }
    let result = (invoke $shell $package ['--place' 'uenv-usr-adhoc'] {UENV_USR_ADHOC: $base})
    let home: path = ($base | path join 'opt' $NAME)
    let link: path = ($bin | path join 'alpha')
    let entry = (ls --long --all $bin | where {|e| ($e.name | path basename) == 'alpha' })
    let target: string = if ($entry | is-empty) { '' } else { $entry | get 0.target | default '' }
    let expected: string = (['..' 'opt' $NAME 'bin' 'alpha'] | path join)
    let checks: list<record<what: string, held: bool>> = [
        {what: $"exit ($result.exit): ($result.err)", held: ($result.exit == 0)}
        {what: 'the home is the place opt/<name>', held: (($result.last | path expand) == ($home | path expand))}
        {what: 'bin/alpha is a link', held: (($entry | length) == 1 and (($entry | get 0.type) == 'symlink'))}
        {what: $"the link names ($expected), not ($target)", held: ($target == $expected)}
        {what: 'bin/beta is linked', held: (($bin | path join 'beta') | path exists)}
    ]
    let unix_checks: list<record<what: string, held: bool>> = if $unix {
        let ran: record<stdout: string, stderr: string, exit_code: int> = (^$link | complete)
        [
            {what: 'the link runs', held: (($ran.stdout | str trim) == 'alpha')}
            {what: "a dangling link into the home is cleared", held: (not ((ls --all $bin | get name | each {|n| $n | path basename }) | any {|n| $n == 'gamma' }))}
            {what: "another's link stays", held: ((ls --all $bin | get name | each {|n| $n | path basename }) | any {|n| $n == 'other' })}
        ]
    } else { [] }
    checked 'a uenv install links its bin' ($checks ++ $unix_checks)
}

def case_bin_file [shell: string, package: path, scratch: path]: nothing -> record<case: string, ok: bool, detail: string> {
    let base: path = ($scratch | path join 'adhoc-file')
    mkdir ($base | path join 'bin')
    "mine\n" | save ($base | path join 'bin' 'beta')
    refused 'a file in the bin' (invoke $shell $package ['--place' 'uenv-usr-adhoc'] {UENV_USR_ADHOC: $base}) 'is not a link'
}

def case_foreign_home [shell: string, package: path, scratch: path]: nothing -> record<case: string, ok: bool, detail: string> {
    let home: path = ($scratch | path join 'foreign')
    mkdir $home
    "keep\n" | save ($home | path join 'keep.txt')
    let result = (invoke $shell $package ['--place' 'dir' '--dir' $home] {})
    let refusal = (refused 'a foreign home refused' $result 'no earlier install')
    if not (($home | path join 'keep.txt') | path exists) {
        {case: $refusal.case, ok: false, detail: 'its file was removed'}
    } else {
        $refusal
    }
}

def case_tampered [shell: string, package: path, scratch: path]: nothing -> record<case: string, ok: bool, detail: string> {
    let copy: path = (copy_package $package ($scratch | path join 'tampered'))
    let tarball: path = ($copy | path join $"($NAME).tar")
    open --raw $tarball | into binary | bytes add --end 0x[00] | save --force $tarball
    refused 'a tampered tar refused' (invoke $shell $copy ['--place' 'dir' '--dir' ($scratch | path join 'tampered-home')] {}) 'does not match its record'
}

def case_unknown_place [shell: string, package: path]: nothing -> record<case: string, ok: bool, detail: string> {
    refused 'an unknown place' (invoke $shell $package ['--place' 'nowhere'] {}) 'unknown place'
}

def case_no_place [shell: string, package: path]: nothing -> record<case: string, ok: bool, detail: string> {
    refused 'no place' (invoke $shell $package [] {}) '--place is required'
}

def case_unknown_argument [shell: string, package: path]: nothing -> record<case: string, ok: bool, detail: string> {
    refused 'an unknown argument' (invoke $shell $package ['--bogus'] {}) 'unknown argument'
}
