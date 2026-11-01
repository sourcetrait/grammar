#!/usr/bin/env bash
# grammar-mcp's package tree, built on this machine in a one-time podman container.
#
# Usage: `bash run.bash --sources <dir> --out <dir> [--base <image>]
# [--rust <version>] [--jobs <n>]`, on the machine the tree is for.
#
# `--sources` holds grammar_mcp beside the repositories it reaches: common,
# common_mono, srcert, bitcode, and gram. The image is built from this
# directory's Containerfile on `--base`, this machine's own distribution, and
# runs build.bash, which leaves the tree in `<out>/tree` and its build record
# in `<out>/build.nuon`. The image, the base when this run pulled it, and the
# build's work directory beside `--out` are removed afterwards, whatever the
# outcome. The last line is the tree's path.

set -euo pipefail

repos=(grammar_mcp common common_mono srcert bitcode gram)
base=registry.fedoraproject.org/fedora:44
rust=1.99.1
jobs=4
sources=
out=

while (($#)); do
    case "$1" in
        --sources) sources="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        --base) base="$2"; shift 2 ;;
        --rust) rust="$2"; shift 2 ;;
        --jobs) jobs="$2"; shift 2 ;;
        *) echo "run.bash: unknown argument: $1" >&2; exit 2 ;;
    esac
done
if [[ -z "$sources" || -z "$out" ]]; then
    echo "run.bash: needs --sources and --out" >&2
    exit 2
fi
sources="$(realpath -e "$sources")"
for repo in "${repos[@]}"; do
    if [[ ! -d "$sources/$repo" ]]; then
        echo "run.bash: no $repo in $sources" >&2
        exit 1
    fi
done
mkdir -p "$out"
out="$(realpath -e "$out")"

context="$(dirname "$(realpath -e "${BASH_SOURCE[0]}")")"
stamp="$(date -u +%Y%m%d%H%M%S)"
image="localhost/grammar-mcp-build:$stamp"
work="$(dirname "$out")/.grammar-mcp-build-$stamp"
pulled=false
if ! podman image exists "$base"; then
    pulled=true
fi
mkdir "$work"

cleanup() {
    podman rmi --force "$image" >/dev/null 2>&1 || true
    if [[ "$pulled" == true ]]; then
        podman rmi "$base" >/dev/null 2>&1 || true
    fi
    rm -rf "$work"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

mounts=()
for repo in "${repos[@]}"; do
    mounts+=(--volume "$sources/$repo:/src/$repo:ro,Z")
done

podman build --tag "$image" --file "$context/Containerfile" \
    --build-arg "BASE=$base" --build-arg "RUST=$rust" "$context"
podman run --rm --init --env "JOBS=$jobs" "${mounts[@]}" \
    --volume "$work:/work:Z" --volume "$out:/out:Z" \
    "$image" bash /src/grammar_mcp/build/container/build.bash
echo "$out/tree"
