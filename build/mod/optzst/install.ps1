#!/usr/bin/env pwsh
# An OptZst package's install: its inner tar checked and unpacked into a home.
#
# Usage: `pwsh install.ps1 --place <place> [--dir <path>]`, beside the
# package's members, `<name>.tar` and its `.sha256` record, with the
# `package.nuon` naming the package.
#
# `--place uenv-usr-local` and `--place uenv-usr-adhoc` install into
# `$UENV_USR_LOCAL/opt/<name>` or `$UENV_USR_ADHOC/opt/<name>` and link each
# entry of its `bin/` from that home's `bin/`, which on Windows takes an
# elevated session; `--place dir --dir <path>` installs into `<path>`, linking
# nothing. A home already holding files is emptied only when its
# `package.nuon` names this package, an earlier install. The last line is the
# home's path.

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Fail([string] $Message) {
    [Console]::Error.WriteLine("install.ps1: $Message")
    exit 1
}

if ($PSVersionTable.PSVersion.Major -lt 7) {
    Fail "PowerShell 7 or later is required, this is $($PSVersionTable.PSVersion)"
}

$place = $null
$dir = $null
$dirGiven = $false
for ($i = 0; $i -lt $args.Count; $i++) {
    switch -CaseSensitive ($args[$i]) {
        '--place' {
            if ($i + 1 -ge $args.Count) { Fail '--place needs a value' }
            $i++
            $place = [string] $args[$i]
        }
        '--dir' {
            if ($i + 1 -ge $args.Count) { Fail '--dir needs a value' }
            $i++
            $dir = [string] $args[$i]
            $dirGiven = $true
        }
        default { Fail "unknown argument: $($args[$i])" }
    }
}

$namePattern = '(^|[{,\s])name:\s*"?([A-Za-z0-9.-]+)'
function Get-PackageName([string] $Path) {
    $found = [regex]::Match((Get-Content -Raw -LiteralPath $Path), $namePattern)
    if ($found.Success) { $found.Groups[2].Value } else { $null }
}

# An item at a path, a dangling link included, or nothing.
function Get-Entry([string] $Path) {
    Get-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
}

$here = $PSScriptRoot
$record = Join-Path $here 'package.nuon'
if (-not (Test-Path -LiteralPath $record -PathType Leaf)) { Fail 'no package.nuon beside install.ps1' }
$name = Get-PackageName $record
if (-not $name) { Fail 'package.nuon names no package' }

$bin = $null
if (-not $place) { Fail '--place is required: uenv-usr-local, uenv-usr-adhoc, or dir' }
switch -CaseSensitive ($place) {
    { $_ -in 'uenv-usr-local', 'uenv-usr-adhoc' } {
        if ($dirGiven) { Fail '--dir belongs to --place dir alone' }
        $variable = if ($place -eq 'uenv-usr-local') { 'UENV_USR_LOCAL' } else { 'UENV_USR_ADHOC' }
        $base = [Environment]::GetEnvironmentVariable($variable)
        if (-not $base) { Fail "$variable is not set" }
        $optHome = Join-Path $base 'opt' $name
        $bin = Join-Path $base 'bin'
        break
    }
    'dir' {
        if (-not $dir) { Fail '--place dir needs --dir' }
        $optHome = $dir
        break
    }
    default { Fail "unknown place: $place" }
}

if ($bin -and $IsWindows) {
    $principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Fail 'run elevated: the links into the bin are symlinks, which Windows makes for an administrator'
    }
}

$tarball = Join-Path $here "$name.tar"
$sums = "$tarball.sha256"
if (-not ((Test-Path -LiteralPath $tarball -PathType Leaf) -and (Test-Path -LiteralPath $sums -PathType Leaf))) {
    Fail "no $name.tar and its record beside install.ps1"
}
$expected = ((Get-Content -Raw -LiteralPath $sums).Trim() -split '\s+')[0].ToLowerInvariant()
$actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $tarball).Hash.ToLowerInvariant()
if ($actual -ne $expected) { Fail "$name.tar does not match its record" }

$existing = Get-Entry $optHome
if ($existing) {
    if ($existing.LinkType) { Fail "$optHome is a link, not a directory" }
    if (-not $existing.PSIsContainer) { Fail "$optHome is not a directory" }
    $children = @(Get-ChildItem -LiteralPath $optHome -Force)
    if ($children.Count -gt 0) {
        $marker = Get-Entry (Join-Path $optHome 'package.nuon')
        $earlier = if ($marker -and -not $marker.LinkType -and -not $marker.PSIsContainer) { Get-PackageName $marker.FullName } else { $null }
        if ($earlier -ne $name) { Fail "$optHome holds files that are no earlier install of $name" }
        $children | Remove-Item -Recurse -Force
    }
} else {
    New-Item -ItemType Directory -Path $optHome -Force | Out-Null
}
& tar -xf $tarball -C $optHome
if ($LASTEXITCODE -ne 0) { Fail "tar could not unpack $name.tar" }
Copy-Item -LiteralPath $record -Destination (Join-Path $optHome 'package.nuon')

if ($bin) {
    New-Item -ItemType Directory -Path $bin -Force | Out-Null
    $separator = [IO.Path]::DirectorySeparatorChar
    $targetRoot = '..', 'opt', $name, 'bin' -join $separator
    foreach ($link in @(Get-ChildItem -LiteralPath $bin -Force)) {
        $target = $link.LinkTarget
        if ($target -and $target.StartsWith("$targetRoot$separator")) {
            $resolved = [IO.Path]::GetFullPath((Join-Path $bin $target))
            if (-not ([IO.File]::Exists($resolved) -or [IO.Directory]::Exists($resolved))) {
                Remove-Item -LiteralPath $link.FullName -Force
            }
        }
    }
    foreach ($entry in @(Get-ChildItem -LiteralPath (Join-Path $optHome 'bin') -Force)) {
        $link = Join-Path $bin $entry.Name
        $present = Get-Entry $link
        if ($present) {
            if (-not $present.LinkType) { Fail "$link exists and is not a link" }
            Remove-Item -LiteralPath $link -Force
        }
        New-Item -ItemType SymbolicLink -Path $link -Target (Join-Path $targetRoot $entry.Name) | Out-Null
    }
}
Write-Output $optHome
