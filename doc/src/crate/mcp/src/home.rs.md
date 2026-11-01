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

## const TMP_DEFAULT
Where the temporary directory sits when neither `UENV_USR_TMP` nor `TMPDIR`
names one, as Rust's own `temp_dir` has it.

## static BASE_DIRS
`directories::BaseDirs` honors `$XDG_CACHE_HOME` and `$XDG_DATA_HOME`, and with
the UENV variables in front of them that is what lets every in-process test
isolate its state under a per-binary tempdir: the harness sets all four before
the first access here (`guts.rs`).

It is `None` where no home directory can be found. `serve_or_oneshot` runs
`check_base_dirs` before anything reads it and exits 2 without one, so
`base_dirs`, the one reader, finds it set.

## fn env_dir
The XDG spec has a relative path in its variables ignored, and the UENV homes
and `TMPDIR` are held to the same. An empty value reads as unset, and a
relative one is reported as ignored, so a misconfiguration shows rather than
quietly moving the files.

## static CACHE_ROOT / static DATA_ROOT
Read once, as `BASE_DIRS` is, so a relative variable is reported once rather
than at every path built under it, and every path agrees with the first.

## fn Config::cache_home
The config's id first, then its namespace, so an agent's whole state across
every namespace is one subtree to wipe or back up. The vendor and app segments
above that keep the tree from colliding with anything else under the home. The
data home is built the same way under its own root (`Config::data_home`).
