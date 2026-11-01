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

## const WINDOWS_SYSTEM_PID
The System process, pid 4, is the home of the kernel-mode system threads and
appears on every Windows system. Its creation time is read from the kernel's
process records, which open no handle to it.

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

### fn write
`create_new` and no symlink followed, so the marker is a file of this process's
making inside the directory it just made. It is 0600 on Unix; on Windows it
takes the ACL the directory passes down, which the directory took from its
base.

### fn read
A predecessor's marker is read only where the guard's is absent, and the two
are held to the same checks after.

### fn checked
A pid of 0 names the caller's process group to a signal, and a negative one
every process it may signal: a marker naming either names no one process, and
its pid never reaches a signal.

## fn current_uid
On Unix the effective UID, the one a file the process creates is owned by.
Windows names users by SID, so there every owner is 0, the guard counting on
the scratch base there being the user's own, as the system's temporary
directory under the user's profile is: a SID check in the UID's place would
guard nothing more. Every marker written on Windows names UID 0, so a
sweep's check of the marker's UID passes for each.

## fn current_boot
Linux's `/proc/sys/kernel/random/boot_id`, which the kernel draws anew at each
boot; macOS's `kern.bootsessionuuid`, likewise each boot's own. The sysctl
writes at most the buffer's length into the buffer and sets the length it
wrote, the name a NUL-terminated literal, so the call reads and writes only
memory the function owns; a longer answer than the buffer is no boot.
Windows' is the System process's creation time (`WINDOWS_SYSTEM_PID`), in
`sourcetrait_process`'s unit there, 100 ns since 1601. Where the process table
can't be read the boot is none, and every owner is then unknown.

## fn current_pid_namespace
Read as the link's own text, `pid:[<inode>]`, compared whole.

## fn random_nonce
`getrandom::fill`, each platform's own generator: the `getrandom` system call
on Linux, `/dev/urandom` on a kernel without it, `getentropy` on macOS,
`ProcessPrng` on Windows 10 and later. Windows keeps no `/dev/urandom` to read.

## fn read_marker
Opened without following a symlink, on Unix without blocking, then checked
through that handle as a regular file, so a FIFO or a device in a directory is
refused rather than waited on, and nothing can be swapped in between the check
and the read. On Windows the open takes a reparse point as itself, and a link
that opens is refused by the check.
