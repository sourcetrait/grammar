#!/usr/bin/env bash
# grammar's rpm, its source assembled with the network, rebuilt without one,
# and installed in a fresh system, each in a one-time podman container.
#
# Usage: `bash run.bash --sources <dir> --out <dir> [--base <image>]
# [--jobs <n>]`, on a Fedora machine of the architecture the rpm is for.
#
# `--sources` holds grammar beside common, common_mono, srcert, and gram. The
# image is built from this directory's Containerfile on `--base`; srpm.bash
# writes the source rpm into `<out>`; `rpmbuild --rebuild` runs on it with no
# network, its rpm copied into `<out>`; and the rpm is installed on `--base`
# itself, also with no network, each binary run. Each phase's log lands in
# `<out>`. The image, the base when this run pulled it, and the work directory
# beside `--out` are removed afterwards, whatever the outcome. `--rust`, which
# the server build takes, is accepted and unused: Fedora's own toolchain
# builds, as copr's does. The last line is the rpm's path.

set -euo pipefail

repos=(grammar common common_mono srcert gram)
base=registry.fedoraproject.org/fedora:44
jobs=4
sources=
out=

while (($#)); do
    case "$1" in
        --sources) sources="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        --base) base="$2"; shift 2 ;;
        --jobs) jobs="$2"; shift 2 ;;
        --rust) shift 2 ;;
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
image="localhost/grammar-rpm:$stamp"
work="$(dirname "$out")/.grammar-rpm-$stamp"
pulled=false
if ! podman image exists "$base"; then
    pulled=true
fi
mkdir "$work" "$work/srpm" "$work/rebuild"

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

podman build --tag "$image" --file "$context/Containerfile" --build-arg "BASE=$base" "$context" \
    > "$out/image.log" 2>&1
podman run --rm --init "${mounts[@]}" --volume "$work/srpm:/work:Z" --volume "$out:/out:Z" \
    "$image" bash /src/grammar/build/rpm/srpm.bash > "$out/srpm.log" 2>&1
podman run --rm --init --network none --env "RPM_BUILD_NCPUS=$jobs" \
    --volume "$work/rebuild:/work:Z" --volume "$out:/out:Z" "$image" \
    bash -c 'rpmbuild --rebuild --define "_topdir /work" /out/*.src.rpm && cp /work/RPMS/*/*.rpm /out/' \
    > "$out/rebuild.log" 2>&1
podman run --rm --init --network none --volume "$out:/out:ro,Z" "$base" \
    bash -c 'rpm -i /out/grammar-*.$(uname -m).rpm && grammar-mcp --version && grammar-ssh --version && /opt/grammar/nu/bin/nu --version' \
    > "$out/install.log" 2>&1
ls "$out"/grammar-*."$(uname -m)".rpm
