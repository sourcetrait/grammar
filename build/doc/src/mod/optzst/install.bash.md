# install.bash

An OptZst package's install for Linux and macOS, generic to any package: it
reads the package's name from `package.nuon` and finds `<name>.tar` and its
record beside itself, where the package's members were extracted.

## The shell
bash 5 and later alone; an older bash is refused before anything else runs.
macOS's own `/bin/bash` is 3.2, so a Mac runs it with a newer bash.

## --place and --dir
`uenv-usr-local` and `uenv-usr-adhoc` name a UENV home by its variable,
`UENV_USR_LOCAL` or `UENV_USR_ADHOC`, an unset one refused: the package goes to
its `opt/<name>`, and each entry of the package's `bin/` is linked from the
home's `bin/`. `dir` takes `--dir <path>`, the package going there and nothing
linked; `--dir` with any other place is refused.

## The name
`package.nuon` is NUON, which bash can't parse, so the script reads the `name`
field alone by a pattern anchored to a key's position; OptZst names hold
letters, digits, dots, and dashes, never `_`.

## The record
`<name>.tar` is checked against its sha256 record, by `sha256sum` or, on
macOS, `shasum -a 256`, before a byte of it is unpacked. The signatures are
the consumer's to check with a key it trusts; the script carries no key.

## The home
A home that holds files is emptied only when it is an earlier install of the
same package, its `package.nuon` naming it; anything else is refused, so a
mistaken `--dir` can't empty a directory. The tree is unpacked into the emptied
home, and the package's `package.nuon` is copied in as the install's mark.

## The links
Each link is relative, `../opt/<name>/bin/<entry>`, so the home and its bin
move together. A link the bin holds into this package's `bin/` whose target is
gone, an entry a newer version dropped, is removed; a regular file where a
link would go is refused, never overwritten.
