# install.nu

The install scripts' cases, run under one shell at a time, so each script is
held to the same behaviour on every host that has its shell.

## The fixture
A package as the scripts meet it, built in a scratch directory: an inner tar of
a `bin/` and a `share/`, its sha256 record, a `package.nuon`, and the two
scripts copied from the module. It carries no signature, since the scripts
check the record alone.

## def invoke
Each case runs the script in a process of its own, with its variables set
by `with-env`; a variable set empty reads as unset, which both scripts treat
alike.

## The cases
A fresh install and a reinstall over an earlier one; the refusals, `--dir`
missing or out of place, a UENV variable unset, a regular file in the bin, a
foreign home, a tampered tar, an unknown place, no place, and an unknown
argument; and a UENV install's links, relative and runnable, a dangling link
into the package cleared and another package's left. A foreign home's files
must survive its refusal.
