# install.nu

The install scripts' cases, one OS's script at a time on its own OS, bash's on
Linux, zsh's on macOS, and pwsh's on Windows, so the three are held to the
same behaviour; each script refuses another OS's host.

## The fixture
A package as the scripts meet it, built in a scratch directory: an inner tar of
a `bin/` and a `share/`, its sha256 record, a `package.nuon`, and the three
scripts copied from the module. It carries no signature, since the scripts
check the record alone. The tar is made with short options, which macOS's
bsdtar reads as GNU tar does.

## def invoke
Each case runs the script in a process of its own, with its variables set
by `with-env`; a variable set empty reads as unset, which the scripts treat
alike. zsh runs with `-f`, so the user's `.zshenv` leaves the case's variables
as they are set.

## The cases
A fresh install and a reinstall over an earlier one; the refusals, `--dir`
missing or out of place, a UENV variable unset, a regular file in the bin, a
foreign home, a tampered tar, an unknown place, no place, and an unknown
argument; and a UENV install's links, relative and runnable, a dangling link
into the package cleared and another package's left. A foreign home's files
must survive its refusal.
