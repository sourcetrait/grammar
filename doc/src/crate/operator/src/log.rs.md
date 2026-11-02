# log.rs

The emergency log both hosts keep, the MCP and grammar-ssh alike: each
emergency one NUON line in `emergency.nuonl` under the host's cache home, the
same line and the same place on each, so either host's log reads the same. The
log is the host's policy's to write (the MCP's `server/emergency.rs`,
grammar-ssh's `emergency.rs`); this module holds what they share.

## fn int
Every count is a u64 and the log's ints are i64; a count past i64's range is
held at its ceiling rather than wrapping negative.

## fn emergency_fields
Shared by the log line and the announced record, so the durable record and the
notification can never disagree about what a condition reported. That sharing
is also what made a `from` event field easy to get wrong: the Channel packet's
envelope already carries one, which is why the spam pair names its offender
`origin`.

## fn announced_fields
## fn announced_record
An emergency's fields as the log and the Channel carry them, a remote engine's
led by `host`, its alias, so an agent and the log alike know which machine
raised it.

## fn emergency_line
A remote engine's line names its `host` beside the logging host's own
`mcp_nom`, the MCP keeping every engine's emergencies in its one log.

Every field is a flat scalar, but a string field can still carry a newline - a
spam error's `action` holds an error's message - and NUON writes a newline
inside a string raw. The line therefore escapes `\n` and `\r` as the Channel
escapes a packet's, so every record of `emergency.nuonl` stays on one line; the
escapes read back as the original characters, a compact render's raw newlines
sitting only inside quoted strings.

## fn emergency_log_path
Under the namespace's cache home, the host's. The nom namespaces it per process
because the cache home is only `(id, namespace)`-scoped, so two hosts on one
namespace in turn would otherwise interleave their logs.

## fn append_line
Open-append-close per line: low volume, and robust against an external
truncation between writes. Nothing is buffered to lose. The record and its
newline go in one write, so no other append can land between them.
