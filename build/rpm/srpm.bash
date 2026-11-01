#!/usr/bin/env bash
# grammar's source rpm: every crate the build needs, so a builder with no
# network builds it.
#
# Run by run.bash in its one-time container, with the network: /src holds
# grammar beside common, common_mono, srcert, and gram, read-only, as grammar's
# path dependencies and its equip read them; /work is the run's own; /out
# receives `grammar-<version>.src.rpm`. nushell's and its core plugins' crates
# come from crates.io at the engine's version, and `cargo vendor` fills one
# vendor directory for every manifest the build reads.

set -euo pipefail

repos=(grammar common common_mono srcert gram)
crates=(nu nu_plugin_formats nu_plugin_gstat nu_plugin_inc nu_plugin_polars nu_plugin_query)

for repo in "${repos[@]}"; do
    if [[ ! -d "/src/$repo" ]]; then
        echo "srpm.bash: no $repo in /src" >&2
        exit 1
    fi
done

export CARGO_HOME=/work/cargo
semver="$(cargo pkgid --manifest-path /src/grammar/Cargo.toml -p sourcetrait_grammar_mcp)"
semver="${semver##*@}"
if [[ ! "$semver" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]]; then
    echo "srpm.bash: not a semver: $semver" >&2
    exit 1
fi
version="${semver/-/\~}"
nu_version="$(awk '$0 == "name = \"nu-protocol\"" { getline; gsub(/version = |"/, ""); print; exit }' /src/grammar/Cargo.lock)"
if [[ -z "$nu_version" ]]; then
    echo "srpm.bash: no nu-protocol version in grammar's Cargo.lock" >&2
    exit 1
fi

top="/work/grammar-$version"
mkdir -p "$top/src" "$top/nushell" "$top/cargo-home" /work/rpmbuild/SOURCES /work/rpmbuild/SPECS
for repo in "${repos[@]}"; do
    tar -C /src --exclude="$repo/target" --exclude="$repo/.git" -cf - "$repo" | tar -C "$top/src" -xf -
done
syncs=()
for crate in "${crates[@]}"; do
    curl -fsSL "https://static.crates.io/crates/$crate/$crate-$nu_version.crate" \
        | tar -xzf - -C "$top/nushell"
    syncs+=(--sync "$top/nushell/$crate-$nu_version/Cargo.toml")
done
# The build's copy has its own absolute path, filled in by the spec's %build.
cargo vendor --locked --versioned-dirs --manifest-path "$top/src/grammar/Cargo.toml" \
    "${syncs[@]}" "$top/vendor" \
    | sed "s|$top/vendor|@VENDOR@|" > "$top/cargo-home/config.toml"

tar --zstd -C /work -cf "/work/rpmbuild/SOURCES/grammar-$version.tar.zst" "grammar-$version"
sed -e "s|@VERSION@|$version|g" -e "s|@NU_VERSION@|$nu_version|g" \
    /src/grammar/build/rpm/grammar.spec > /work/rpmbuild/SPECS/grammar.spec
rpmbuild -bs --define "_topdir /work/rpmbuild" /work/rpmbuild/SPECS/grammar.spec
cp /work/rpmbuild/SRPMS/*.src.rpm /out/
ls /out/*.src.rpm
