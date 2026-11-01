# engines.rs

The remote engines' entries, `[[engine]]` in the remotes file beside the peer
links' `[[channel]]` ones, which the MCP reads at each use and writes by its
`interface remote engine` commands (User). An entry may be partial: `set` and
`import` build one a field at a time, so the file holds what was given, and a
session opens on a whole one alone. The peer crate reads the same file for its
channels and owns its locked edit (`crate/peer`'s `remotes.rs`), which these
writes go through.

## const KIND_ENGINE_INVALID
## const KIND_ENGINE_NOT_OPEN
The MCP's own refusals, under `remote::engine::` beside the transport's kinds:
a command's parameters or an entry refused, and a close of no open session.
They render in `error.rs` rather than through the protocol's `RuntimeError`,
which schema v1 fixed.

## const REQUIRED
The fields a session needs, the port and the namespace having defaults.

## struct EnginesToml
The `[[channel]]` entries are read as plain tables and left to the peer, so
`deny_unknown_fields` refuses any other key.

## struct EngineToml
Every field optional, so a partial entry reads; an absent alias is refused by
name.

## struct EngineEntry
The fields as the file writes them, so `list` and `get` show what was written,
a `~` key path among them; the key expands where a session needs it.

### fn check_given
Each field the entry gives held to the client's check for it (`crate/client`'s
`endpoint.rs`), so a bad value is refused at the command that gave it rather
than at a later connect. The key is checked expanded, and the id and the
namespace together, the one given beside the other's default.

### fn missing
The required fields the entry lacks, in the file's order, which a row reports
and the endpoint's refusal names.

### fn endpoint
A whole entry with its defaults, its key expanded, held to the client's
`check`. The command and the workdir are the host's paths, not this one's, so
they never expand.

## fn read_engines
An absent file is no engines, the ordinary state of a host that reaches none.

## fn parse_engines
Each entry held as `check_given` holds it, and an alias names one engine
alone. An engine's alias and a remote channel's never meet in one command, so
the two may share a name.

## fn set_engine
`toml_edit` edits the document in place, so comments, order, formatting, and
the channels stay; a new entry is appended, and only the fields given change.
The result is held to `parse_engines` before it is returned, so a set never
writes a file the next read would refuse.

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

## fn known_hosts_name
OpenSSH's own form for a host on another port, the name `ssh-keygen -F` finds.

## fn import_ssh
OpenSSH resolves the host, so an import reads what `ssh` itself would: its
`Include`s, its `Match` blocks, its defaults, and `-F`'s file in place of the
user's when one is given. The first identity file that exists is the key, ssh
trying each in turn; the host keys are every one the known hosts files hold for
the host, under its `HostKeyAlias` where it names one. A host never connected
to has none, so the import refuses it rather than pin nothing. The command,
the id, and the workdir are the worker's, which no ssh configuration knows, so
an import leaves them to `set`.
