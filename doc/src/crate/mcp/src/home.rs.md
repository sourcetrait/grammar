# home.rs

The host's homes. The engine takes two as its params, the namespace's data home
and its cache home (`server/host.rs`), and the scratch directories are made in
two more (`scratch.rs`).

## const UENV_HOMES
A home is read from its UENV variable first, then from the standard variable
behind it, then from that variable's default. The UENV tree is the user's own
environment, set alike on every machine the user works on, while the standard
variables are whatever a machine's login sets. A UENV tree with no standard
variable, such as `UENV_USR_SECRET`, is not a home the host reads.

## fn tmp_default
Where the temporary directory sits when neither `UENV_USR_TMP` nor `TMPDIR`
names one: `/tmp` on Unix, and elsewhere Rust's `temp_dir`, on Windows the
system's own `GetTempPath2W`, `TMP`, then `TEMP`, then the user's profile. A
fixed `/tmp` on Windows would name `\tmp` on the current drive, which no
Windows system keeps.

## static BASE_DIRS
The platform's own folders, the roots' last resort: XDG's defaults under the
home on Linux, `Library/Caches` and `Library/Application Support` on macOS, and
the local and the roaming application data folders on Windows.

It is `None` where no home directory can be found. `serve_or_oneshot` runs
`check_base_dirs` before anything reads it and exits 2 without one, so
`base_dirs`, the one reader, finds it set.

## fn env_dir
The XDG spec has a relative path in its variables ignored, and the UENV homes
and `TMPDIR` are held to the same. An empty value reads as unset, and a
relative one is reported as ignored, so a misconfiguration shows rather than
quietly moving the files.

## static CACHE_ROOT / static DATA_ROOT
Each root keeps `UENV_HOMES`' order on every platform: its UENV variable, then
XDG's, then the platform's folder. `directories` reads XDG's variables on Linux
alone, so the XDG step is the host's own: left to `directories`, a set
`XDG_DATA_HOME` would be passed over for the platform's folder on macOS and
Windows. With the UENV variables in front, this is what lets every in-process
test isolate its state under a per-binary tempdir: the harness sets all four
before the first access here (`guts.rs`).

Read once, as `BASE_DIRS` is, so a relative variable is reported once rather
than at every path built under it, and every path agrees with the first.

## fn Config::cache_home
The config's id first, then its namespace, so an agent's whole state across
every namespace is one subtree to wipe or back up. The vendor and app segments
above that keep the tree from colliding with anything else under the home. The
data home is built the same way under its own root (`Config::data_home`).
