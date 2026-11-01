# lock.rs

The namespace lock every program that starts an engine takes first: the MCP's
server and one-shot CLI and grammard alike, so one engine works a namespace
whichever program started it. It lives beside the scratch guard because both
products take this crate and its record names a process as a scratch marker
does.

## const LOCK_FILE
The data home's `host.lock`, the name the MCP's operator once held there, so a
watcher reading the old name keeps reading the live one.

## struct LockRecord
Its fields name the holding process under its boot, as a scratch marker's do,
and the holder's nom, so a reader tells which program and which host holds the
namespace; `include/scratch/Lock.nutype` is its model. A worker's generation is
the hello's to carry, never the lock's.

### fn current
The process's start is read as the scratch guard reads it, in the platform's
unit, so a record and a marker of one process agree.

## enum LockError
`Held` carries the holder's record as it wrote it, bounded and on one line, so
a refusal names who holds the namespace; `Unavailable` is a lock that can't be
taken or written for a reason of its own, a symlink, another UID's file, an
I/O failure.

## struct NamespaceLock

### fn acquire
`flock`, exclusive and without waiting, so a second holder fails at once
rather than queueing, and the kernel releases the lock with its holder's last
descriptor, so a killed holder's lock passes to the next start with nothing to
clean. The file is opened without following a symlink and without blocking,
checked as a regular file the UID owns, and never removed or replaced: a
rename would leave the holder locking an inode no later start opens.

### fn record
Written in place over the last holder's, the file truncated rather than
replaced, for the same reason, and synced.

## fn holder_of
At most `HOLDER_MAX` bytes, its whitespace folded to single spaces, so a
refusal's text stays one line whatever the file held.
