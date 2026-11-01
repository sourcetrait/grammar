# install.ps1

An OptZst package's install for Windows, and for Linux and macOS where
PowerShell is installed, the same install as `install.bash`, its arguments
spelled the same.

## The shell
PowerShell 7 and later alone; Windows PowerShell 5.1 is refused. The script
declares no parameters and reads `--place` and `--dir` from `$args` itself, so
both scripts take one command line.

## Elevation
The links into a UENV home's `bin/` are symbolic links, which Windows makes for
an administrator, so on Windows an install to a UENV place refuses an
unelevated session before it changes anything. `--place dir` makes no link and
runs unelevated.

## The home
`$home` is PowerShell's own read-only variable, so the opt home is
`$optHome`. As in `install.bash`, a home holding files is emptied only when its
`package.nuon` names this package, and the tree is unpacked by `tar`, which
Windows ships, then marked with the package's `package.nuon`.

## The links
Each link's target is relative, `../opt/<name>/bin/<entry>` in the platform's
separator; an entry whose link the bin already holds is replaced, a dangling
link into the package's `bin/` is removed, and a regular file is refused.
`Test-Path` answers true for a dangling link, as PowerShell 7.6.9 does on
Linux, so a link's target is read from .NET's `LinkTarget`, resolved against
the bin, and checked by `[IO.File]::Exists` and `[IO.Directory]::Exists`;
`Get-Item -Force` finds a link whatever its target.
