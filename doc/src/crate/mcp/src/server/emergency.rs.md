# emergency.rs

The controller's policy over its systems' emergencies, the operator's and the
Channel's spam pair: each logged, then announced. The conditions themselves are
the operator's model, and the log's line, its path, and its append are the
operator's too (the operator's `log.rs`), grammar-ssh writing the same log.

## fn emergency_model
`mcp/` is a reservation. Every host-originated model lives beneath it, which is
what lets a model path from a foreign source be checked mechanically: anything
claiming `mcp/` that is not the host is not entitled to it. Enforced today at
the one place a non-host picks a model, and available to a future peer surface
on the same terms.

A unit test asserts every variant is under the reservation and that the models
are distinct. One variant escaping would turn a mechanical provenance check
back into a judgement.

## struct EmergencyPolicy
mcp's controller makes it (`controller.rs`). The operator's handler answers the
operator's emergencies with it (`operator.rs`), the face's hub slot raises the
Channel's spam pair to it once supervision installs it (`channel/face.rs`), and
the remote engines' handler raises each grammar-ssh's (`remote.rs`), so one
policy answers every source, into one log and onto one Channel. It holds nothing
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
