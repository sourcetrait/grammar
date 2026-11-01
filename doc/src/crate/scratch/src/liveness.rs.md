# liveness.rs

## trait Probe
The process table as data, so a test answers what a host can't be made to: a
pid held by another user's process, a record that can't be read, a zombie,
another boot, or another namespace.

### fn keeps_pid_namespaces
Apart from `pid_namespace`, so a namespace that can't be read on Linux is never
taken for macOS's having none: on a platform that keeps them, both sides must
name the same one.

## struct HostProbe
### fn process
On Unix, the process table first (`sourcetrait_process`): a record gives the
pid's start. Where none is found the table can't say why, an exit or a
refusal, so a null signal asks: `ESRCH` means no process holds the pid,
`EPERM` that the signal was refused, and a success that a process holds it
whose record can't be read. The signal sends nothing.

Elsewhere no signal asks, and the whole table is read. On Windows it holds
every user's processes, so a pid it lacks is held by no process and is absent.
A table that reads empty is a failed read, since it would hold this very
process, and is unreadable. No answer there is denied, and none is a zombie: a
Windows process leaves the table when it exits.

## enum Liveness
Three answers, since a sweep may act on a proof alone: an answer short of one is
unknown, never dead.

## fn liveness
A pid means something only under the boot and the PID namespace that numbered
it, so an owner is judged only under the sweeper's own. Another boot is an
earlier one of this machine or any boot of another sharing the base, and the
marker can't say which; another namespace numbers its pids apart, a live
owner's pid absent here or another process's; a marker naming neither, a
predecessor's, can be judged under no scope at all. Each is unknown, and a
directory under it is kept. Under the sweeper's own scope, dead is proven three
ways: no process at the pid; a process at the pid with another start, since two
live processes never share a pid; or a zombie, which has exited. A pid whose
process can't be read is unknown, and so is one whose null signal is refused:
another user's process at the pid could not own this user's directory, but a
sandbox or a security module refuses a signal to this user's own processes
too, and the refusal can't tell the two apart.
