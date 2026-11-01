# teardown.rs

On Unix, nushell 0.116.1 runs each external of an `is_mcp` engine in its own
session, `setsid` before exec, so the external leads a process group of its own
(`engine.rs`); on Windows it starts the external with `CREATE_NO_WINDOW` alone,
in no group of its own. A `ThreadJob` tracks only that direct child's pid, and
a descendant that starts its own session or group leaves the external's, so
neither the tracked pid nor a killpg reaches the whole tree: a plain kill of
the child orphans its grandchild.

The reap that works: a descendant walk of the process table
(`sourcetrait_process`, `/proc` on Linux, libproc on macOS, and the kernel's
process records on Windows) from each tracked child before the kill, which
makes it deterministic, plus a kill of each. On Linux the host is also the
eval's subreaper, so an orphan reparents to it and stays on its ppid chain (the
operator's `reaper.rs`). `Jobs::kill_and_remove` alone reaches only the tracked
direct child.

On macOS the table holds the host user's own processes, which is every process an eval
starts, and there is no subreaper: a descendant orphaned before the walk goes to
launchd and escapes it. Where no table is read the walk finds nothing, and the kill
reaches the tracked children alone.

On Windows the table holds every process, and none is reparented: a process
keeps its parent's pid after that parent exits. The walk starts from each
tracked pid, so a tracked child's own children are found even after it exits,
but a deeper descendant whose parent exited before the walk escapes it, and an
orphan whose parent's pid a later process took is passed over, being older
than that process (`sourcetrait_process::descendants_in`).

## fn tree_kill
The table is read once and walked BEFORE the kill, which is what makes the set
deterministic rather than a race against reparenting.

## fn make_tracker
Setting the returned job as an engine's `current_job.background_thread_job` is what makes
nushell register the pid of every external that eval spawns into an Arc-shared set, which
the kill and timeout paths then read.

The Mail sender is a THROWAWAY on purpose: pid tracking is sender-independent, and a
foreground eval never emits job Mail.

Its `Signals` share the eval's cancel flag, so once cancel fires no further pid is
registered.

## fn kill_plugin_subprocesses
Unblocks a plugin-hung eval, which nothing else can: a hung plugin ignores the cancel
`Signals` and is not a tracked external, so killing its subprocess is what closes the
plugin IPC pipe and makes the eval's plugin read return an error. Plugins respawn lazily
on next use.

BROAD BY NATURE - nushell shares plugin subprocesses across evals - so it fires only on
an explicit `Kill` and on the teardown, NEVER on an automatic timeout, where the
supervisor scopes that escalation instead.

## fn kill_pid
`SIGKILL` on Unix. On Windows the process is opened with `PROCESS_TERMINATE`
alone and ended by `TerminateProcess`, the handle closed after; a pid that
cannot be opened is gone or not ours, and is passed over as a failed kill is
on Unix. Neither reaches the process's children; `tree_kill`'s walk does.
