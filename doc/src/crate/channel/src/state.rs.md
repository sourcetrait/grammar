# state.rs

The logical Channel: its phase, its mode, its epoch, its claim, its sink, its
inbox, and its spam. The system owns it and runs one step at a time, so nothing
here locks; what a step sends up queues in `ups` and goes up after it.

## enum UpKind
What the Channel sends its controller unasked, each under its epoch: a line for
an async hub, the hub's close frame, and an emergency. Each goes up before the
step's answer, so a line reaches its hub, and a close its frame, before the
sender hears the result.

## enum Sink
One send path for both modes: `emit` and `send_control` push to whichever sink
the open installed, so the gate, the spam counter, the spill, and the render
never ask the mode. An async Channel's lines go up for the face's hub; a sync
one's queue here, bounded at `SYNC_QUEUE_CAP`, the drops counted for the next
sync.

### variant Gone
The hub's peer left. The logical Channel stays open until the agent closes it,
and a line answers `HubGone`, so a send and a re-greeting both learn the peer is
gone.

## struct Channel
### field epoch
Kept past a close until the next open, so a stale notice or expiry finds the
open it concerns already ended.

### field inbox
The host tree's `channel/` (`scratch.rs`'s `make_channel_dir`), flat: every
packet whole as `<id>.nuon` and its files in `<id>/`, whoever sent it, so the
agent reads any packet's files by its id alone. A remote link's files are staged
under `.incoming/` and moved under the relayed packet's id, so a relayed
packet's layout is a local one's. A deliberate `interface channel close` takes
it and prunes it; every other teardown leaves it. The shutdown close, the
verify-timer expiry and the unverified-emit teardown are all host-initiated, so
a packet's files stay readable by its id when the channel went away without the
caller asking for it, until the host itself ends; the next open takes the
directory as it stands.

### fn open
A sync channel binds nothing: its greeting is the first line queued, and its
claim starts unset, as a fresh hub's does, so the same verification gate holds
it.

### fn take_sync
Taking is the claim. A sync channel has no connection to claim it, so the first
sync, which hands the agent the greeting through the stdio session it drives,
sets the claim a hub's accept would; `interface channel ready` then verifies as
it does for a Monitor. A refused sync claims nothing.

### fn hub_notice
The face's hub claims and loses its peer on its own tasks, so each notice names
its open's epoch, and one for another open changes nothing: a slow notice from a
closed hub cannot claim or orphan the open that replaced it.

### fn mark_verified
Verification requires a claim. The handshake asserts the agent owns the
claiming connection, so with no claim there is nothing to own. It also closes a
real hole: a verified-but-unclaimed channel would accept emits into a queue
nothing drains. The refusal names the mode, since what the agent does next
differs: start a Monitor, or sync.

### fn close_if_unverified
The phase and the epoch are read here, at the expiry, because verification can
land between the timer's sleep elapsing and its request arriving, and a later
open can replace the one the timer was armed for; a bare `close` would tear down
a channel that had just proven itself.

### fn close_now
The teardown every close shares. An async Channel's close sends its hub the
frame, which the hub writes on a lane of its own, so a planned close never waits
behind queued traffic and the agent never sees the bare 1006 a crash looks
like.

### fn send_control
Bypasses the gate, and `emit` does not. That split is load-bearing rather than
a convenience: the gate keeps telemetry off an unverified peer, but the
handshake packet is the thing the peer verifies itself by seeing, so it
necessarily precedes verification.

### fn emit
The phase is read at the moment of sending, never captured: a job outliving its
eval keeps everything it closed over, so a spawn-time snapshot would let work
started before verification emit to an unproven peer.

### fn plan_send
A body's send, its order deliberate. The `mcp/` and `remote/` refusal comes
first, before the phase is read: a definition is not the host, so letting it
stamp a model under either prefix would let it forge a host packet and lose the
property the reservation exists for, that provenance is checkable from the path
alone. A peer's relay is held to the same check (`link.rs`). A model past
`MODEL_MAX_CHARS` is refused beside it, so a spilled line, which keeps the
model, always fits the cap.

Open-but-unverified does not merely refuse - it tears the channel down.
Telemetry must not reach a peer that has not proven it owns the stdio session,
and a peer that has been emitted to unproven is not a peer we keep.

On the hard threshold the send is refused with a stop: the sender stops its own
eval, its signals and its job, and reports what it did, which rides the error's
emergency (`stopped`).

The id is minted before the files are named because their names derive from
it, which is also why the hash covers the attached content rather than its
path. The packet is written whole as `<id>.nuon` whatever its size, so every
packet is on disk; a line past the cap goes out spilled, `{size}` alone, and
the agent reads the packet by its id (`render.rs`). The attachment is
`<id>/attached.nuon`, and the packet's `attached` table names it with its size:
a big event and an attachment both survive, in separate files.

The plan stops at the files, which the system writes before it finishes the
send (`system.rs`).

### fn finish_send
The gate is read again at the emit, as `emit` always reads it; nothing between
the plan and the finish changes it, since the system runs one step at a time.

### fn plan_emit
A host packet's send, a relay's or a report's, gated as `emit` is. Its files are
already on disk, a peer's staged by the link, so the id is minted over their
names rather than their content, and their sizes come from the link. A staged
directory moves to `<id>/`. Where the peer spilled its event, the event the
packet carries is only the peer's pointer, so the packet can't be rendered here:
the plan hands the system a `Compose`, which renders it once the moves land.

### fn finish_emit
As `finish_send`, the gate read again at the emit.

## struct Compose
A relay whose peer spilled its event. The spill is read and parsed rather than
copied in, so `<id>.nuon` holds a packet nushell reads whatever the peer sent,
and a spill that won't parse refuses the relay, its files left staged. The spill
goes once taken in, and `<id>/` with it where it was the one file; the line is
the packet's own, held to the cap as every line is.

## fn apply_moves
Renames within the inbox, so a peer's files land whole under the packet's id or
not at all, and the moves go before the writes. The directories the moves
emptied go after them: `<id>/` when nothing was left in it, and the peer's
staging directory once its send's directory has left it.

## fn write_files
Each file's directory is made before its write, so an inbox removed from under
an open Channel comes back rather than failing the send.
