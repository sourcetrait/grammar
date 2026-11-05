#!/bin/zsh -f
# An OptZst package's install on macOS: its inner tar checked and unpacked into
# a home.
#
# Usage: `zsh -f install.zsh --place <place> [--dir <path>]`, beside the
# package's members, `<name>.tar` and its `.sha256` record, with the
# `package.nuon` naming the package. `-f` keeps the user's startup files from
# changing the environment the install reads.
#
# `--place uenv-usr-local` and `--place uenv-usr-adhoc` install into
# `$UENV_USR_LOCAL/opt/<name>` or `$UENV_USR_ADHOC/opt/<name>` and link each
# entry of its `bin/` from that home's `bin/`; `--place dir --dir <path>`
# installs into `<path>`, linking nothing. A home already holding files is
# emptied only when its `package.nuon` names this package, an earlier install.
# The last line is the home's path.

emulate -L zsh
setopt err_exit no_unset pipe_fail

fail() {
    print -ru2 -- "install.zsh: $*"
    exit 1
}

if (( ${ZSH_VERSION%%.*} < 5 )); then
    fail "zsh 5 or later is required, this is $ZSH_VERSION"
fi
[[ "$(uname -s)" == Darwin ]] || fail "install.zsh is macOS's: Linux takes install.bash, Windows install.ps1"

here="${0:A:h}"
place=
dir=
dir_given=false
while (( $# )); do
    case "$1" in
        --place)
            (( $# >= 2 )) || fail "--place needs a value"
            place="$2"
            shift 2
            ;;
        --dir)
            (( $# >= 2 )) || fail "--dir needs a value"
            dir="$2"
            dir_given=true
            shift 2
            ;;
        *) fail "unknown argument: $1" ;;
    esac
done

name_pattern='(^|[{,[:space:]])name:[[:space:]]*"?([A-Za-z0-9.-]+)'
package_name() {
    local content
    content="$(<"$1")"
    [[ "$content" =~ $name_pattern ]] || return 1
    print -r -- "$match[2]"
}

record="$here/package.nuon"
[[ -f "$record" ]] || fail "no package.nuon beside install.zsh"
name="$(package_name "$record")" || fail "package.nuon names no package"

bin=
case "$place" in
    uenv-usr-local | uenv-usr-adhoc)
        [[ "$dir_given" == false ]] || fail "--dir belongs to --place dir alone"
        if [[ "$place" == uenv-usr-local ]]; then
            base="${UENV_USR_LOCAL:-}"
            [[ -n "$base" ]] || fail "UENV_USR_LOCAL is not set"
        else
            base="${UENV_USR_ADHOC:-}"
            [[ -n "$base" ]] || fail "UENV_USR_ADHOC is not set"
        fi
        home="$base/opt/$name"
        bin="$base/bin"
        ;;
    dir)
        [[ -n "$dir" ]] || fail "--place dir needs --dir"
        home="$dir"
        ;;
    "") fail "--place is required: uenv-usr-local, uenv-usr-adhoc, or dir" ;;
    *) fail "unknown place: $place" ;;
esac

tarball="$here/$name.tar"
sums="$tarball.sha256"
[[ -f "$tarball" && -f "$sums" ]] || fail "no $name.tar and its record beside install.zsh"
command -v shasum >/dev/null 2>&1 || fail "shasum is not installed"
read -r expected rest <"$sums"
digest="$(shasum -a 256 "$tarball")"
[[ "${digest%% *}" == "$expected" ]] || fail "$name.tar does not match its record"

if [[ -L "$home" ]]; then
    fail "$home is a link, not a directory"
elif [[ -e "$home" ]]; then
    [[ -d "$home" ]] || fail "$home is not a directory"
    if [[ -n "$(ls -A "$home")" ]]; then
        marker="$home/package.nuon"
        earlier=
        if [[ -f "$marker" && ! -L "$marker" ]]; then
            earlier="$(package_name "$marker")" || earlier=
        fi
        [[ "$earlier" == "$name" ]] || fail "$home holds files that are no earlier install of $name"
        find "$home" -mindepth 1 -maxdepth 1 -exec rm -rf -- {} +
    fi
else
    mkdir -p "$home"
fi
tar -xf "$tarball" -C "$home"
cp "$record" "$home/package.nuon"

if [[ -n "$bin" ]]; then
    mkdir -p "$bin"
    target_root="../opt/$name/bin"
    for link in "$bin"/*(N); do
        if [[ -L "$link" && "$(readlink "$link")" == "$target_root/"* && ! -e "$link" ]]; then
            rm -f -- "$link"
        fi
    done
    for entry in "$home"/bin/*(N); do
        [[ -e "$entry" ]] || continue
        link="$bin/${entry:t}"
        if [[ -e "$link" && ! -L "$link" ]]; then
            fail "$link exists and is not a link"
        fi
        ln -sfn "$target_root/${entry:t}" "$link"
    done
fi
print -r -- "$home"
