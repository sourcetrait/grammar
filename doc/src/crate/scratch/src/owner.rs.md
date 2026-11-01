# owner.rs

## const MARKER_FILE
NUON, held to `include/scratch/Owner.nutype`, as every file the worker keeps
is.

## const LEGACY_MARKER_FILE
The MCP's marker before the guard, `<pid> <start>` on one line. It names no
UID, so the directory's owner stands for it, and neither a boot nor a PID
namespace, so no sweep can judge its owner.

## const MARKER_MAX
A marker is a few dozen bytes; the bound keeps a sweep's read of a file in a
shared base small whatever the file holds.

## struct Owner
A pid alone is recycled; with its start it names one process, but only within
the boot and the PID namespace that numbered it. The boot sets apart this
machine's earlier boots and every other machine sharing a base; the namespace
sets apart processes in other containers on this one, whose pids are numbered
on their own.

### field pidns
Linux's `/proc/self/ns/pid` link, the namespace's inode. It names one namespace
within a boot, the boot naming the kernel; every initial namespace shares one
inode across machines, which the boot sets apart.

### field nonce
Each directory's own, so a guard's removal can tell its directory from another
made at the same path: it removes only a directory whose marker still holds its
nonce.

## fn current
The nonce's sixteen bytes come from `/dev/urandom`, which every platform the
guard builds on keeps.

## fn write
`create_new` and no symlink followed, so the marker is a file of this process's
making inside the directory it just made.

## fn read
A predecessor's marker is read only where the guard's is absent, and the two
are held to the same checks after.

## fn checked
A pid of 0 names the caller's process group to a signal, and a negative one
every process it may signal: a marker naming either names no one process, and
its pid never reaches a signal.

## fn current_boot
Linux's `/proc/sys/kernel/random/boot_id`, which the kernel draws anew at each
boot; macOS's `kern.bootsessionuuid`, likewise each boot's own. The sysctl
writes at most the buffer's length into the buffer and sets the length it
wrote, the name a NUL-terminated literal, so the call reads and writes only
memory the function owns; a longer answer than the buffer is no boot.

## fn current_pid_namespace
Read as the link's own text, `pid:[<inode>]`, compared whole.

## fn read_marker
Opened without following a symlink and without blocking, then checked through
that handle as a regular file, so a FIFO or a device in a directory is refused
rather than waited on, and nothing can be swapped in between the check and the
read.
