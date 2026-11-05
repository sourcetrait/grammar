# grammar's tree for Windows in a release pipeline: built here by this machine's own toolchain
# and left in -Out as grammar.tar.

param(
    [Parameter(Mandatory)] [string] $Sources,
    [Parameter(Mandatory)] [string] $Out,
    [int] $Jobs = 4
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$env:CARGO_BUILD_JOBS = "$Jobs"
New-Item -ItemType Directory -Force -Path $Out | Out-Null
& nu --no-config-file (Join-Path $Sources 'grammar' 'build' 'bin' 'build.nu') --tree $Out
tar -C $Out -cf (Join-Path $Out 'grammar.tar') package install build.nuon
