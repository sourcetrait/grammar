#!/usr/bin/env bash
# grammar's package tree, built in its one-time container.
#
# Run by run.bash beside it: /src holds grammar beside the repositories it
# reaches, read-only; /work holds cargo's home and target; /out receives the
# tree and its build record from `build.nu --tree`. JOBS is cargo's build jobs.

set -euo pipefail

repo=/src/grammar
export CARGO_HOME=/work/cargo
export CARGO_TARGET_DIR=/work/target
export CARGO_BUILD_JOBS="${JOBS:-4}"

nu_version="$(awk '$0 == "name = \"nu-protocol\"" { getline; gsub(/version = |"/, ""); print; exit }' "$repo/Cargo.lock")"
if [[ -z "$nu_version" ]]; then
    echo "no nu-protocol version in $repo/Cargo.lock" >&2
    exit 1
fi

cargo install --locked --root "$CARGO_TARGET_DIR/nu/release" \
    --target-dir "$CARGO_TARGET_DIR/nu/build" "nu@$nu_version"
"$CARGO_TARGET_DIR/nu/release/bin/nu" --no-config-file "$repo/build/bin/build.nu" --tree /out
