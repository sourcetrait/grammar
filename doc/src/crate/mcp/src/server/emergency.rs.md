# emergency.rs

The controller's policy over its systems' emergencies, the operator's and the
Channel's spam pair: each logged, then announced. The conditions themselves are
the operator's model (the operator's `emergency.rs`).

## fn emergency_model
`mcp/` is a reservation. Every host-originated model lives beneath it, which is
what lets a model path from a foreign source be checked mechanically: anything
claiming `mcp/` that is not the host is not entitled to it. Enforced today at
the one place a non-host picks a model, and available to a future peer surface
on the same terms.

A unit test asserts every variant is under the reservation and that the models
are distinct. One variant escaping would turn a mechanical provenance check
back into a judgement.

## fn emergency_record
Shared by the log line and the channel packet, so the durable record and the
notification can never disagree about what a condition reported. That sharing
is also what made a `from` event field easy to get wrong: the packet's envelope
already carries one, which is why the spam pair names its offender `origin`.

## fn announced_record
An emergency's fields as the log and the Channel carry them, a remote engine's
led by `host`, its alias, so an agent and the log alike know which machine
raised it.

## fn emergency_line
A remote engine's line names its `host` beside the MCP's own `mcp_nom`, the MCP
keeping every engine's emergencies in its one log.

Every field is a flat scalar, but a string field can still carry a newline - a spam
error's `action` holds an error's message - and `to nuon` writes a newline inside a
string raw. The render therefore goes through `escape_line`, as a packet's does, so
every record of `emergency.nuonl` stays on one line; the escapes read back as the
original characters, a compact render's raw newlines sitting only inside quoted
strings.

## fn emergency_log_path
Under the namespace's cache home, the config's. `mcp_nom` namespaces it per
process because the cache home is only `(id, namespace)`-scoped, so two
concurrent hosts on one namespace would otherwise clobber each other's log.

## fn append_line
Open-append-close per line: low volume, and robust against an external truncation
between writes. Nothing is buffered to lose. The record and its newline go in one
write, so no other append can land between them.

## struct EmergencyPolicy
mcp's controller makes it (`controller.rs`). The operator's handler answers the
operator's emergencies with it (`operator.rs`), the face's hub slot raises the
Channel's spam pair to it once supervision installs it (`channel/face.rs`), and
the remote engines' handler raises each worker's (`remote.rs`), so one policy
answers every source, into one log and onto one Channel. It holds nothing
beyond its handles, so each source holds a clone.

### fn new
The log's path is a parameter: the serve path's is the host's per-process log
(`emergency_log_path`), and a test's is a file of its own.

### fn respond
The MCP's own emergency, through `respond_from`.

### fn respond_from
The log is written first and unconditionally, so the durable record never
depends on a channel being open; the announcement follows it. A remote
engine's emergency comes from the remote engines' handler (`remote.rs`), its
host its alias.

### fn append
The operator's handler and the Channel's per-emergency tasks append at once,
through clones of one policy, so the clones share one lock and take their
appends one at a time; the announcement stays outside it. A record's one write
alone keeps its own bytes together, and the lock keeps a partial write, should
the file take one, from letting another record in.

### fn announce
Through the Channel's gated emit (`EmitRequest`), so it is verification-gated
like any other telemetry: an unproven peer must not receive host state. A
closed or unverified channel silently drops the announcement, which is correct
rather than an error: channels are optional and the log already holds the
record.

No rate limiting is applied or needed. The operator's level gates upstream
already bound each condition to roughly one report per episode.
