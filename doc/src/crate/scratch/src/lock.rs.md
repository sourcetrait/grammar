# lock.rs

The namespace lock every program that starts an engine takes first: the MCP's
server and one-shot CLI and grammar-ssh alike, so one engine works a namespace
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
the hello's to carry, never the lock's. On Windows its `uid` is 0, as
`current_uid` answers there.

### fn current
The process's start is read as the scratch guard reads it, in the platform's
unit, so a record and a marker of one process agree.

## enum LockError
`Held` carries the holder's record as it wrote it, bounded and on one line, so
a refusal names who holds the namespace; where the holder's lock bars the
read, as on Windows, it carries the read's failure instead. `Unavailable` is a
lock that can't be taken or written for a reason of its own, a symlink,
another UID's file on Unix, an I/O failure.

## struct NamespaceLock
Its file is nix's `Flock` on Unix, which unlocks on drop, and a plain `File`
elsewhere, whose lock the system releases when the handle closes.

### fn acquire
The file is opened without following a symlink, on Unix without blocking too,
and checked as a regular file, on Unix one the UID owns; it is never removed or
replaced: a rename would leave the holder locking an inode no later start
opens. A symlink is refused on Unix at the open, as
`it does not open as a plain file`; on Windows, where the open takes the link
itself, a link that opens is refused at the check, as
`it is not a regular file`. Windows names owners by SID, which `current_uid`
does not read, so no owner is checked there.

### fn lock
`flock` on Unix, exclusive and without waiting, so a second holder fails at
once rather than queueing, and the kernel releases the lock with its holder's
last descriptor, so a killed holder's lock passes to the next start with
nothing to clean. A `flock` is advisory, so a contender still reads the
holder's record.

Elsewhere `File::try_lock`: on Windows `LockFileEx`, exclusive and failing at
once, over the whole file from offset 0. That lock bars every other handle's
reads and writes of the file, the contender's among them, so a refusal there
carries no record. Windows releases a dead holder's lock too, but its
documentation leaves the moment to the system's resources, so a start right
after a holder's death may still be refused.

### fn record
Written in place over the last holder's, the file truncated rather than
replaced, for the reason `acquire` never replaces it, and synced.

## fn holder_of
At most `HOLDER_MAX` bytes, its whitespace folded to single spaces, so a
refusal's text stays one line whatever the file held. A read that fails is
named as such, `a holder whose record can't be read:` and its error, never
taken for a holder that wrote no record: on Windows the holder's lock bars the
read.
