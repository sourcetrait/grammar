# dir.rs

## const DIR_MODE
Asked of `tempfile` outright, so the umask can only narrow it.

## struct OwnedDir
### fn create
`tempfile`'s builder, as `mkdtemp` makes one: the name unguessable and the mode
0700 from creation. The marker follows the directory; a sweep meeting the
directory between the two finds no marker and keeps it, and a marker that
can't be written takes the directory with it.

### fn remove
The directory is checked before it goes: a directory still, holding this
guard's nonce. A path that was replaced, by a symlink or another guard's
directory, is refused and left. `remove_dir_all` follows no symlink inside it.
