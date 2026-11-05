#!/usr/bin/env zsh
# grammar's tree for macOS in a release pipeline: built here by this machine's own toolchain,
# which its ~/.zshenv puts on the PATH, and left in --out as grammar.tar.

set -euo pipefail

sources=
out=
jobs=4
while (($#)); do
    case "$1" in
        --sources) sources="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        --jobs) jobs="$2"; shift 2 ;;
        *) print -r -- "macos.zsh: unknown argument: $1" >&2; exit 2 ;;
    esac
done
[[ -n "$sources" && -n "$out" ]] || { print -r -- "macos.zsh: needs --sources and --out" >&2; exit 2 }

CARGO_BUILD_JOBS="$jobs" nu --no-config-file "$sources/grammar/build/bin/build.nu" --tree "$out"
COPYFILE_DISABLE=1 tar --no-mac-metadata -C "$out" -cf "$out/grammar.tar" package install build.nuon
