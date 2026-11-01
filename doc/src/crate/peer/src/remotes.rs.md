# remotes.rs

The RemoteChannel's entries in a file of their own, `remotes.toml` beside the
agent's config file, so the remotes change on the fly: every remote channel
command reads the file anew, and `interface remote channel set` and `unset`
write it. The identity stays in the config file, read once at the start. An
entry is a `[[channel]]`, naming its kind, so a remote engine's entries can sit
beside them.

## const DEFAULT_LISTEN
Loopback, never every interface: a listen with no address names the one machine
alone, and an agent that wants more names `0.0.0.0` itself.

## struct RemotesToml
The `[[engine]]` entries are the MCP's (`crate/mcp`'s `engines.rs`), read here
as plain tables and left alone, so `deny_unknown_fields` still refuses any
other key.

## struct RemoteToml
`deny_unknown_fields`, so a retired field, `public_key_file` or a per-link key
pair, fails loudly. Its names are the operator's, the entry being about the
peer, so `cert` is the peer's.

## struct RemoteEntry
The fields as the file writes them, so `list` and `get` show what the operator
wrote, a `~` path among them; each is checked when read and parsed again where
it is used.

### fn peer_cert
The certificate's path expands where a link needs it, `~` from `$HOME` and
each `$VAR` from the environment, an unset one from the stand-ins its
controller passed as data (`ExpandDefault`), the same the controller's own
expansion takes (`crate/mcp`'s `config.rs`), so a path reads alike on either
side.

## fn bind_address
A bare `ip` binds port 0, the kernel's choice, so a `listen` without a port
listens on a random one.

## fn read_remotes
An absent file is no remotes, the ordinary state of a host that links nothing.

## fn set_entry
`toml_edit` edits the document in place, so the operator's comments, order,
and formatting stay; a new entry is appended. The result is held to
`parse_remotes` before it is returned, so a set never writes a file the next
read would refuse: a new entry without its `cert` is refused here.

## fn unset_entry
Comments written above the entry go with it, `toml_edit` keeping a table's
leading decor as the table's own.

## const REMOTES_LOCK
The file an edit locks on Windows, beside the remotes file and never removed.

## fn lock_target
On Unix the remotes file's directory, opened as it is. Windows opens no
directory as a plain file and locks none, so there a file beside the remotes
file, `remotes.lock`, takes the lock, which no rename of the remotes file
touches either.

## fn edit_remotes
Two hosts may share the file, `grammar` and `grammar-test` starting in one
directory, so the read, the edit, and the write run under an exclusive lock on
`lock_target`'s file: a lock on the remotes file itself would not survive the
rename that replaces it. The new text is written to a temporary file in the same
directory, synced, given the old file's permissions, and renamed over it, so a
reader sees the old file or the new one, never a torn write. It is public: the
MCP's engine commands write their entries through it, so a channel's write and
an engine's take the one lock.
