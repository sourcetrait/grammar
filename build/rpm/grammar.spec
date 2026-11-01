# grammar's rpm: the self-contained tree under /opt/grammar, built offline from
# a source rpm that carries every crate the build needs.
#
# srpm.bash beside this file fills in the version and the engine's nushell
# version and assembles the source; the build runs cargo offline against the
# vendored crates, as build/container/build.bash runs it online.

%global debug_package %{nil}
%global nu_version @NU_VERSION@

Name:           grammar
Version:        @VERSION@
Release:        1%{?dist}
Summary:        LLM harness toolkit powered by Nushell
License:        AGPL-3.0-or-later
URL:            https://sourcetrait.com
Source0:        grammar-%{version}.tar.zst
ExclusiveArch:  x86_64 aarch64 riscv64

BuildRequires:  cargo
BuildRequires:  clang
BuildRequires:  cmake
BuildRequires:  gcc
BuildRequires:  gcc-c++
BuildRequires:  make
BuildRequires:  openssl-devel
BuildRequires:  perl
BuildRequires:  pkgconf-pkg-config
BuildRequires:  rust

%description
Grammar's MCP server and its SSH worker, with nushell and its plugins, as one
self-contained tree under /opt/grammar. Nothing outside it is required.

%prep
%autosetup -n grammar-%{version}

%build
sed -i "s|@VENDOR@|$PWD/vendor|" cargo-home/config.toml
export CARGO_HOME="$PWD/cargo-home"
export CARGO_TARGET_DIR="$PWD/target"
export CARGO_NET_OFFLINE=true
cargo install --offline --locked --root "$CARGO_TARGET_DIR/nu/release" \
    --target-dir "$CARGO_TARGET_DIR/nu/build" --path "nushell/nu-%{nu_version}"
"$CARGO_TARGET_DIR/nu/release/bin/nu" --no-config-file \
    src/grammar/build/bin/build.nu --tree "$PWD/out" --nu-sources "$PWD/nushell"

%install
mkdir -p %{buildroot}/opt/grammar %{buildroot}%{_bindir}
cp -a out/tree/. %{buildroot}/opt/grammar/
for path in %{buildroot}/opt/grammar/bin/*; do
    name="$(basename "$path")"
    ln -s "/opt/grammar/bin/$name" "%{buildroot}%{_bindir}/$name"
done

%files
%license src/grammar/LICENSE-AGPL-3.txt src/grammar/LICENSES-EXTERNAL.txt
/opt/grammar
%{_bindir}/grammar-mcp
%{_bindir}/grammar-ssh
