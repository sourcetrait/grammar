# lib.rs

The guard over owned scratch directories and the namespace lock, shared by
every process that keeps them: the MCP and the worker. It holds neither the
engine nor the operator, so any product links it without either.

## mod dir, liveness, owner, sweep
The owned directory, its owner's liveness, the marker, and the sweep.

## mod lock
The namespace lock and its holder's record.

## fn no_follow
On Unix `O_NOFOLLOW` fails the open where the path's last component is a
symlink, `ELOOP` on Linux, and `O_NONBLOCK` keeps the open of a FIFO from
waiting for its other end. Windows opens the link instead:
`FILE_FLAG_OPEN_REPARSE_POINT` (0x00200000) opens a reparse point itself rather
than its target, and std reads a symlink or a junction so opened as a symlink,
never a regular file, so the caller's file-type check refuses it. On Unix the
open itself refuses a link; on Windows a link that opens is refused by the
type check after it.

## fn private_file
The mode is asked at the open, so a file the open creates is 0600 from its
first moment, the umask only narrowing it; a file already there keeps its own
mode. Windows has no mode: a file created there takes the ACL its directory
passes down, so it is as private as that directory.
