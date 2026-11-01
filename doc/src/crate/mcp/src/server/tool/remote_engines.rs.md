# remote_engines.rs

`interface remote engine list` and `get`, and the row every engine command
that answers one answers with.

## struct RemoteEngineRow
An entry's fields as the file writes them beside its session's state, so one
row shows what is configured, what a session still needs, and whether one is
open. A field left to its default is null, as the file leaves it out, and the
known hosts are a list or a file's path, as written. `configured` tells an
entry from a session the file no longer names, whose fields are null.

### fn of
An alias with no entry takes the default entry, every field null.

## fn engine_invalid
The refusal of a command's parameters or of the entry it names.

## fn NuSh::engine_entries
The remotes file read whole at each command, so an edit from elsewhere, a
second host's or a hand's, shows at once.

## fn NuSh::engine_rows
The file's entries and every session not closed, by alias: an entry unset
while its session lives still lists, so it can be closed. A closed session the
file no longer names is gone.

## fn NuSh::engine_row
An alias neither in the file nor live is refused, as a remote channel's is.
