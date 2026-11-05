# scratch.rs

A host's scratch, made at its start and removed at its end: a bulk directory,
read by every definition as `$env.EQUIP_TMP`, and the host's tree under the
runtime directory, `grammar/<nom>/`, read as `$env.EQUIP_TRANSIENT`, a
definition's small runtime files beside the reserved `owner.nuon` and the
Channel's `channel/`. The host makes them rather than reading directories from
the environment, so it works on any machine whatever its environment's
conventions. The split follows the XDG Base Directory spec, which keeps larger
files out of `XDG_RUNTIME_DIR`, since it may sit in memory that cannot be
swapped out. The tree is named for the host's nom, which no other live host
holds, so another process finds a host's files by a name it already has.

## const SCRATCH_PREFIX
The bulk directory's prefix and the sweep's match: a directory not carrying it
is never a scratch directory, so the sweep cannot touch anything else in a
shared base. The runtime base is swept by it too, for the transient
directories hosts made there before the trees.

## const HOST_TREES
Every entry of `grammar/` is a host's tree, so its sweep takes no prefix.

## const SCRATCH_MODE
Asked of `tempfile` and `DirBuilder` outright: `tempfile`'s directories default
to 0777 less the umask, which left one 0755 under a 022 umask. The umask can
only narrow the mode asked for, never widen it. Unix alone: Windows keeps no
mode bits, and a directory made there takes the ACL its parent passes down.

## struct ScratchDirs
The bulk directory beside the tree. Both always exist, so a definition reads
either without asking which the host could make.

## fn make_scratch_dirs
The nom is the engine's identity's, drawn by `init_engine` before the engine
starts (`server/host.rs`), since the tree is made before the engine mints
anything. The bulk directory goes in the temporary directory, `UENV_USR_TMP`,
`TMPDIR`, or the platform's own; the tree in the runtime directory,
`UENV_USR_TRANSIENT` or `XDG_RUNTIME_DIR`, or, where neither names one, beside
the bulk one with a warning, the fallback the XDG spec asks for. Both are read
as every home is (`home.rs`). Each base is swept first, so a host's start is
what cleans after a host killed outright, which runs no exit hook; a base the
two share is swept once, and `grammar/` is swept whole once it is checked, a
dead owner's tree going with its Channel's files. A base that does not exist
fails the start: a platform's directory is checked, never made.

A tree that cannot be made takes the bulk directory with it, so a failed start
leaves neither.

Once a process: a second call returns the first directories, so a second start,
which the engine refuses, leaves none behind.

## fn tmp_dir / fn transient_dir
`None` before the start, which a caller reports as the host having no scratch
directory.

## fn make_channel_dir
One `channel/` a host, in its own tree: a teardown other than a deliberate
close leaves it for the agent to read, and the next open takes it as it stands,
each packet's id keeping its files apart from the last open's. A link in its
place is refused, since only a directory itself passes.

## fn remove_channel_dir
A deliberate close's removal, `.incoming/` with it.

## fn remove_scratch_dirs
Every ordinary end calls it, the client's, the test driver's, the one-shot's, a
failed start's, and a signal's (`run.rs`, `server/channel/face.rs`). The tree
goes whole, the Channel's files in it; a directory already gone is no error. A
host killed outright leaves its tree for the next start's sweep.

## fn make_trees / fn check_trees
`grammar/` is a fixed name, which a shared base such as `/tmp`, the fallback's,
lets anyone make first. One already there is taken only as a directory itself,
on Unix this user's and closed to others' writes: whoever may write it may
rename a host's tree and put a link in its place, and the host would then write
its files wherever the link points. Anything else fails the start, naming the
directory. On Windows it takes the ACL its base passes down.

## fn make_dir
The shared guard's owned directory: the name unguessable, so no other process
can claim it first, and on Unix the mode 0700 from creation, so no other user
can enter it; on Windows it takes the ACL its base passes down. Its owner is
marked with the guard's `owner.nuon`, which names the boot and the PID
namespace a sweep judges the owner under. Where this process's start can't be
read no marker is written, as on a platform with no process table, and a sweep
keeps the directory unmarked.

## fn make_tree
The guard's named directory, its marker as `make_dir`'s; a nom's name already
taken is refused rather than reused. Unmarked where this process's start can't
be read, as `make_dir`'s.

## fn private_tempdir
An unmarked bulk directory's, where the guard can't mark one. `keep` hands the
directory over, since a `TempDir` removes its directory on drop and this one
lives until the process's end.

## fn sweep_stale
The shared guard's sweep (`sourcetrait_grammar_scratch`), grammar-ssh's own: a
directory goes only when its marker's owner is proven gone and, on Unix, the
UID owns it, and a process that can't be read is unknown, never dead. A
directory without a readable marker is kept: it may be a host's still between
its creation and its marker. Each directory kept with no running owner is
reported on stderr.
