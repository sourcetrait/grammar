# dir.rs

## const DIR_MODE
Asked of `tempfile` outright, so the umask can only narrow it. Unix alone:
Windows keeps no mode bits, and a directory made there takes the ACL its base
passes down.

## struct OwnedDir
### fn create
`tempfile`'s builder, as `mkdtemp` makes one: the name unguessable, and on Unix
the mode 0700 from creation. On Windows the directory is made with no security
descriptor of its own, so its ACL is the one its base passes down and it is as
private as the base. The marker follows the directory; a sweep meeting the
directory between the two finds no marker and keeps it, and a marker that
can't be written takes the directory with it.

### fn remove
The directory is checked before it goes: a directory still, holding this
guard's nonce. A path that was replaced, by a symlink or another guard's
directory, is refused and left. `remove_dir_all` follows no symlink inside it.
