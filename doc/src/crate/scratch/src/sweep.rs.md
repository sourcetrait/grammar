# sweep.rs

## struct SweepReport
### fn diagnostics
A running owner's directory is kept without a word; every other kept one is
reported, its path and its reason, since it stays until something proves its
owner gone.

## fn sweep
A base is shared, by other users, other programs, and other guards, so a sweep
judges only its prefix's entries and removes only on a proof. A removal another
sweep finished first is no failure.

## fn judge
The checks run cheapest first, and every one keeps the directory when it
fails: a symlink, never followed; not a directory; another UID's, on Unix; no
readable marker; a marker naming another UID; an owner not proven dead. On
Windows a directory's owner is a SID, which neither std's metadata nor
`current_uid` reads, so no directory's owner is checked there, and a marker
written there names UID 0, which the marker check passes.
