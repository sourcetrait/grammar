#!/usr/bin/env bash
# grammar's runtime cleanup installed for this user: the package's tmpfiles rule
# linked into the user manager's user-tmpfiles.d/, and systemd's own cleaning
# timer enabled, so what sits under the runtime directory's grammar/ and ai/ is
# removed once three days old.
#
# Usage: bash <home>/sys/systemd/install.bash, once after an install, as the
# user the cleanup serves. A rerun relays the link; a package's update needs
# none, the link naming the rule in its home.

set -euo pipefail

name=grammar.conf
timer=systemd-tmpfiles-clean.timer
here="$(dirname "$(realpath -e "${BASH_SOURCE[0]}")")"
rule="$here/$name"

fail() {
    echo "install.bash: $*" >&2
    exit 1
}

[[ -f "$rule" ]] || fail "no $name beside this script, in $here"
command -v systemctl >/dev/null || fail "no systemctl: this host runs no systemd"
manager="$(systemctl --user show-environment 2>/dev/null)" ||
    fail "no user systemd manager for $(id -un): log in through a session that starts one"
systemctl --user cat "$timer" >/dev/null 2>&1 ||
    fail "the user systemd manager has no $timer"

# The user manager's config home, where its systemd-tmpfiles reads user-tmpfiles.d/.
config="$(sed -n 's/^XDG_CONFIG_HOME=//p' <<<"$manager")"
config="${config:-$HOME/.config}"
[[ -d "$config" ]] || fail "the user manager's config home is missing: $config"
dir="$config/user-tmpfiles.d"
link="$dir/$name"

if [[ -L "$link" ]]; then
    target="$(readlink -- "$link")"
    [[ "$target" == */sys/systemd/"$name" ]] ||
        fail "$link links to $target, not a grammar home's rule, so it stays"
    rm -- "$link"
elif [[ -e "$link" ]]; then
    fail "$link is not a grammar home's link, so it stays: move it aside first"
fi
mkdir -p -- "$dir"
ln -s -- "$rule" "$link"
systemctl --user enable --now "$timer"
echo "$link -> $rule"
