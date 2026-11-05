# install.zsh

An OptZst package's install for macOS, generic to any package: it reads the
package's name from `package.nuon` and finds `<name>.tar` and its record beside
itself, where the package's members were extracted. It is `install.bash`'s
install in zsh, the shell macOS ships, its arguments spelled the same.

## The shell
zsh 5 and later alone, on macOS alone: a host whose `uname -s` isn't `Darwin`
is refused, naming `install.bash` for Linux and `install.ps1` for Windows.
macOS 27 ships zsh 5.9 at `/bin/zsh`, its users' login shell.

The script runs under `zsh -f`, its shebang `#!/bin/zsh -f`: zsh reads a
user's `~/.zshenv` for every zsh it starts, a script's included, and a
`.zshenv` setting the UENV homes would replace the variables the caller
passes. `emulate -L zsh` then sets zsh's own options, and `err_exit`,
`no_unset`, and `pipe_fail` hold the script to the bash one's `-euo pipefail`.

## --place and --dir
As `install.bash` takes them: `uenv-usr-local` and `uenv-usr-adhoc` name a UENV
home by its variable, an unset one refused, the package going to its
`opt/<name>` and each entry of its `bin/` linked from the home's `bin/`; `dir`
takes `--dir <path>`, nothing linked.

## The name
The `name` field is read by the same anchored pattern as `install.bash` reads
it, zsh's `=~` giving its groups in `$match`.

## The record
`<name>.tar` is checked against its sha256 record by `shasum -a 256`, which
macOS ships, before a byte of it is unpacked.

## The home
As `install.bash`'s: a home holding files is emptied only when its
`package.nuon` names this package, and the tree is unpacked by macOS's tar,
then marked with the package's `package.nuon`.

## The links
As `install.bash`'s, relative and checked the same way. zsh refuses a glob that
matches nothing, so the bin's entries and the home's are globbed with the `(N)`
qualifier, an empty directory giving an empty list; `${entry:t}` is an entry's
name. The home's path is the last line, written by `print -r`, since zsh's
`echo` reads escapes.
