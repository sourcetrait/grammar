#!/usr/bin/env bash
# An OptZst package's install: its inner tar checked and unpacked into a home.
#
# Usage: `bash install.bash --place <place> [--dir <path>]`, beside the
# package's members, `<name>.tar` and its `.sha256` record, with the
# `package.nuon` naming the package.
#
# `--place uenv-usr-local` and `--place uenv-usr-adhoc` install into
# `$UENV_USR_LOCAL/opt/<name>` or `$UENV_USR_ADHOC/opt/<name>` and link each
# entry of its `bin/` from that home's `bin/`; `--place dir --dir <path>`
# installs into `<path>`, linking nothing. A home already holding files is
# emptied only when its `package.nuon` names this package, an earlier install.
# The last line is the home's path.

set -euo pipefail

fail() {
    echo "install.bash: $*" >&2
    exit 1
}

if ((BASH_VERSINFO[0] < 5)); then
    fail "bash 5 or later is required, this is $BASH_VERSION"
fi

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
place=
dir=
dir_given=false
while (($#)); do
    case "$1" in
        --place)
            (($# >= 2)) || fail "--place needs a value"
            place="$2"
            shift 2
            ;;
        --dir)
            (($# >= 2)) || fail "--dir needs a value"
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
    printf '%s\n' "${BASH_REMATCH[2]}"
}

record="$here/package.nuon"
[[ -f "$record" ]] || fail "no package.nuon beside install.bash"
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
[[ -f "$tarball" && -f "$sums" ]] || fail "no $name.tar and its record beside install.bash"
read -r expected _ <"$sums"
if command -v sha256sum >/dev/null 2>&1; then
    read -r actual _ < <(sha256sum "$tarball")
elif command -v shasum >/dev/null 2>&1; then
    read -r actual _ < <(shasum -a 256 "$tarball")
else
    fail "neither sha256sum nor shasum is installed"
fi
[[ "$actual" == "$expected" ]] || fail "$name.tar does not match its record"

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
    for link in "$bin"/*; do
        if [[ -L "$link" && "$(readlink "$link")" == "$target_root/"* && ! -e "$link" ]]; then
            rm -f -- "$link"
        fi
    done
    for entry in "$home"/bin/*; do
        [[ -e "$entry" ]] || continue
        entry_name="$(basename "$entry")"
        link="$bin/$entry_name"
        if [[ -e "$link" && ! -L "$link" ]]; then
            fail "$link exists and is not a link"
        fi
        ln -sfn "$target_root/$entry_name" "$link"
    done
fi
echo "$home"
