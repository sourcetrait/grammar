# error.rs

## enum DaemonError
Plain data with a hand-written `Display`, so an embedding host matches on a
variant and grammard prints the same text it exits by. The instance's refusals,
a name, a workdir, a config file, or a home, each carry what they refused, so a
report names the very name, path, or home at fault.

## fn DaemonErrorKind::name
The kind as a report or a test names it: the instance's under `instance::`,
scratch's under `scratch::`, the start's own under `runtime::`, and a session
that couldn't open under `session::`.

## fn exit_code
sysexits' meanings, so a supervisor or a test tells the failures apart without
reading text: 64, `EX_USAGE`, for a name that is not one; 78, `EX_CONFIG`, for a
workdir, a config file, or a home the start refuses; 73, `EX_CANTCREAT`, for
scratch the worker can't make or remove and a lock it can't take for a reason
of the lock's own; 75, `EX_TEMPFAIL`, for a namespace another process holds,
which a later start may find free; 71, `EX_OSERR`, for task threads the system
won't give; 76, `EX_PROTOCOL`, for a session whose hello was refused or whose
stream broke before it; and 70, `EX_SOFTWARE`, for an engine that fails its
start or a second runtime in one process, a caller's bug.

## impl From<LockError>
The scratch crate's lock refusals as the worker's own kinds, the holder's
record kept, so a held namespace still reports `runtime::namespace_held`.
