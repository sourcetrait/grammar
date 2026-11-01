# engines.rs

The remote engines' entries, `[[engine]]` in the remotes file beside the peer
links' `[[channel]]` ones, which the MCP reads at each use and writes by its
`interface remote engine` commands (User). An entry is aligned with a channel's
(User): an `address`, `known_hosts` a list of keys or a file, and the worker's
`command`, `id`, `namespace`, and `workdir` optional. An entry may be partial:
`set` and `import` build one a field at a time, so the file holds what was
given, and a session opens on a whole one alone. The peer crate reads the same
file for its channels and owns its locked edit (`crate/peer`'s `remotes.rs`),
which these writes go through.

## const KIND_ENGINE_INVALID
## const KIND_ENGINE_NOT_OPEN
The MCP's own refusals, under `remote::engine::` beside the transport's kinds:
a command's parameters or an entry refused, and a close of no open session.
They render in `error.rs` rather than through the protocol's `RuntimeError`,
which schema v1 fixed.

## const REQUIRED
The fields a session needs; every other one has a default.

## const RETIRED
The fields an older entry wrote, `host` and `port` in one `address` now and
`host_keys` in `known_hosts`, each refused by what took its place rather than
as an unknown field, so a file written before reads as a pointer to the new
one.

## const NAME_STAND_IN
An entry giving its id alone, or its namespace alone, is checked beside a
valid name, the absent one's real default known only where a session opens.

## const IMPORT_MARK
The first line of a known hosts file an import writes. An import rewrites a
file opening with it and refuses any other, so a file kept by hand at that
path is never lost.

## struct EnginesToml
The `[[channel]]` entries are read as plain tables and left to the peer, so
`deny_unknown_fields` refuses any other key.

## struct EngineToml
Every field optional, so a partial entry reads; an absent alias is refused by
name. The retired fields read as any value, only to be refused.

## enum KnownHosts
Untagged, as TOML writes it: a list is the host's keys inline, a string a
known hosts file. A row serializes it the same way.

## struct EngineEntry
The fields as the file writes them, so `list` and `get` show what was written,
a `~` key path among them; the key expands where a session needs it.

### fn check_given
Each field the entry gives held to the client's check for it (`crate/client`'s
`endpoint.rs`), so a bad value is refused at the command that gave it rather
than at a later connect. The key is checked expanded, and the id and the
namespace together. A known hosts file is checked as a path alone; it is read
where a session opens, so an entry may name one not yet written.

### fn missing
The required fields the entry lacks, in the file's order, which a row reports
and the endpoint's refusal names.

### fn endpoint
A whole entry resolved: its address split, its key expanded, a known hosts
file read for the address's keys, and the defaults taken, `grammar-ssh` for
the command and the MCP's own id and namespace, then held to the client's
`check`. A file holding no key for the host is refused by its path and the name
it was read for. The command and the workdir are the host's paths, not this
one's, so they never expand.

## struct EngineDefaults
What a session needs from the MCP that an entry leaves out, made from the
config at each use, as the entry itself is read.

## fn check_file
A known hosts path is any text but an empty one or one holding a control
character; spaces pass, the path never reaching a shell.

## fn known_hosts_path
A path expanded as the config's are, a relative one read beside the remotes
file, where an import writes its own.

## fn file_keys
The file read and matched by the client's reader (`crate/client`'s
`known_hosts.rs`), each refusal naming the file.

## fn read_engines
An absent file is no engines, the ordinary state of a host that reaches none.

## fn parse_engines
Each entry held as `check_given` holds it, and an alias names one engine
alone. An engine's alias and a remote channel's never meet in one command, so
the two may share a name.

## fn engine_endpoint
The endpoint of the entry the file holds for an alias, with the defaults.

## fn set_engine
`toml_edit` edits the document in place, so comments, order, formatting, and
the channels stay; a new entry is appended, and only the fields given change.
A field given drops the retired fields it replaces, so an older entry moves to
the new fields by a set of them. The result is held to `parse_engines` before
it is returned, so a set never writes a file the next read would refuse.

## fn unset_engine
Comments written above the entry go with it, `toml_edit` keeping a table's
leading decor as the table's own.

## struct SshResolved
## fn parse_ssh_g
`ssh -G` prints a lowercase keyword and its value a line; the fields an entry
takes are read, and the user's and the global known hosts files, each
keyword's list split on spaces.

## fn parse_known_keys
`ssh-keygen -F` prints a `#` line before each match. A `@cert-authority` or a
`@revoked` line is skipped: a revoked key is never pinned, and an authority's
key is no host's own. A hashed entry reads as a plain one, its key in the same
two fields.

## fn address_of
The address an import writes, the port left out at 22 as a hand would write
it, and an IPv6 host bracketed as `parse_address` reads it.

## struct SshImport
An import's fields beside the keys it found and the name it writes them under,
the address's own, so the file's lines match the entry's address whatever
`HostKeyAlias` the keys were found under.

## fn import_ssh
OpenSSH resolves the host, so an import reads what `ssh` itself would: its
`Include`s, its `Match` blocks, its defaults, and `-F`'s file in place of the
user's when one is given. The first identity file that exists is the key, ssh
trying each in turn; the host keys are every one the known hosts files hold for
the host, under its `HostKeyAlias` where it names one. A host never connected
to has none, so the import refuses it rather than pin nothing. The command,
the id, the namespace, and the workdir are the worker's, which no ssh
configuration knows, so an import leaves them to their defaults or a `set`.

## fn import_known_hosts
`.grammar/remote/<alias>/known_hosts` beside the remotes file, the alias's
segments its directories (User).

## fn write_known_hosts
The keys under the address's name, a line each after the mark, staged beside
the file and renamed over it, so a reader never sees half a file.
