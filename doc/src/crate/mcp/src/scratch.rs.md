# scratch.rs

A host's scratch directories, two a process, each private to it, made at its
start, and removed at its end: one for bulk, read by every definition as
`$env.EQUIP_TMP`, and one for small runtime files, `$env.EQUIP_TRANSIENT`. The
host makes them rather than reading directories from the environment, so it
works on any machine whatever its environment's conventions, and a path no
other process shares needs no id inside it to keep two hosts apart. The split
follows the XDG Base Directory spec, which keeps larger files out of
`XDG_RUNTIME_DIR`, since it may sit in memory that cannot be swapped out.

Beside the transient one, each Channel open makes the Channel's inbox, a
directory of its own in the same base.

## const SCRATCH_PREFIX
The sweep's match: a directory not carrying it is never a scratch directory, so
the sweep cannot touch anything else in a shared base.

## const CHANNEL_PREFIX
Not the sweep's prefix: an inbox carries no owner marker and no sweep takes
it, its name the host's nom, which no other live host holds, or a random
suffix.

## static CHANNEL_DIRS
Every inbox the process made, not only the Channel's current one: a teardown
other than a deliberate close leaves its inbox for the agent to read, and the
next open makes another, so the end removes each.

## const SCRATCH_MODE
Asked of `tempfile` outright: its directories default to 0777 less the umask,
which left one 0755 under a 022 umask. The umask can only narrow the mode asked
for, never widen it. Unix alone: Windows keeps no mode bits, and a scratch
directory or an inbox made there takes the ACL its parent passes down.

## struct ScratchDirs
The bulk directory beside the transient one. Both always exist, so a definition
reads either without asking which the host could make.

## fn make_scratch_dirs
The bulk directory goes in the temporary directory, `UENV_USR_TMP`, `TMPDIR`,
or the platform's own; the transient one in the runtime directory, `UENV_USR_TRANSIENT` or
`XDG_RUNTIME_DIR`, or, where neither names one, beside the bulk one with a
warning, the fallback the XDG spec asks for. Both are read as every home is
(`home.rs`). Each base is swept first, so a host's start is what cleans after a
host killed outright, which runs no exit hook; a base the two share is swept
once. A base that does not exist fails the start: a platform's directory is
checked, never made.

A transient directory that cannot be made takes the bulk one with it, so a
failed start leaves neither.

Once a process: a second call returns the first directories, so a second start,
which the engine refuses, leaves none behind.

## fn tmp_dir / fn transient_dir
`None` before the start, which a caller reports as the host having no scratch
directory.

## fn make_channel_dir
The base is the transient directory's own, so an inbox follows the same order,
`UENV_USR_TRANSIENT`, `XDG_RUNTIME_DIR`, then beside the bulk one. The nom's
name is made with `create`, which fails on a name already there, so an inbox
left by an earlier open or a directory of anyone else's is never reused; the
fallback is `mkdtemp`'s, under the same prefix, through `private_tempdir`. Both
are made at `SCRATCH_MODE` from creation on Unix; on Windows both take the ACL
the base passes down.

## fn remove_channel_dir
A deliberate close's and a failed start's removal, the path forgotten first: an
inbox the process removed is no longer its own, and a directory that takes its
name later is someone else's, which the end must leave.

## fn remove_scratch_dirs
Every ordinary end calls it, the client's, the test driver's, the one-shot's, a
failed start's, and a signal's (`run.rs`, `server/channel/face.rs`). The
inboxes go first, each one the process made; a directory already gone, an inbox
a deliberate close removed among them, is no error. A host killed outright
leaves its inbox, which nothing sweeps.

## fn make_dir
The shared guard's owned directory: the name unguessable, so no other process
can claim it first, and on Unix the mode 0700 from creation, so no other user
can enter it; on Windows it takes the ACL its base passes down. Its owner is
marked with the guard's `owner.nuon`, which names the boot and the PID
namespace a sweep judges the owner under. Where this process's start can't be
read no marker is written, as on a platform with no process table, and a sweep
keeps the directory unmarked.

## fn private_tempdir
Every randomly named directory the host makes outside the guard comes from
here: an unmarked scratch directory and an inbox whose nom's name is taken.
`keep` hands the directory over, since a `TempDir` removes its directory on
drop and these live until the process's end or the Channel's close.

## fn sweep_stale
The shared guard's sweep (`sourcetrait_grammar_scratch`), the worker's own: a
directory goes only when its marker's owner is proven gone and, on Unix, the
UID owns it, and a process that can't be read is unknown, never dead. A
directory without a readable marker is kept: it may be a host's still between
its creation and its marker. Each directory kept with no running owner is
reported on stderr.
