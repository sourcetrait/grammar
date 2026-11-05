#!/usr/bin/env bash
# grammar's tree for a Linux target of a release pipeline: built by build/container/run.bash,
# whose arguments it takes, and left in --out as grammar.tar.

set -euo pipefail

here="$(dirname "$(realpath -e "${BASH_SOURCE[0]}")")"
out=
words=("$@")
while (($#)); do
    if [[ "$1" == --out ]]; then
        out="$2"
    fi
    shift
done
[[ -n "$out" ]] || { echo "linux.bash: needs --out" >&2; exit 2; }

bash "$here/../container/run.bash" "${words[@]}"
tar -C "$out" -cf "$out/grammar.tar" package install build.nuon
