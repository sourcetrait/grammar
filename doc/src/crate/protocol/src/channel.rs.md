# channel.rs

The Channel's portable data: the reservations and caps every side enforces alike,
its modes, phases, and state, and a send as data. The Channel is the MCP's, and
a remote worker's send reaches it as an ask (session.rs). Closures and borrowed
nushell values never cross: a send crosses as data and its eval's stop stays on
the eval's side.

## const MCP_RESERVED_PREFIX / const REMOTE_RESERVED_PREFIX / fn reserved_prefix
Every model the host stamps lives beneath one of the two: `mcp/` for the local
host's own packets, `remote/` for those about a remote, whose peer need not be
an MCP (User). The value of a reservation is that it makes provenance a
mechanical check: anything claiming either prefix from a source that is not the
host can be rejected without interpreting it. Enforced wherever a non-host picks
a model, a definition's send and a peer's relay, both through `reserved_prefix`.

## const MAX_FRAME_BYTES
A hard `<`, never a `<=` against 1 MiB. A frame landing exactly on the cap
arrives missing its first byte - not an error and not a dropped event, but a
malformed record that fails `from nuon` at the reader with nothing upstream to
blame. Above the cap the frame is dropped whole and the watch closes.

## const SYNC_QUEUE_CAP
A sync channel's queue waits on the agent, so it is bounded where the hub's,
drained as fast as the socket takes it, is not: the oldest packet goes past the
cap and the drop is counted for the next sync, so an agent learns what it missed
rather than nothing. A packet is a notification, never a payload, and the spam
thresholds already bound a definition's rate, so a thousand covers a long
silence.

## const NOTIFICATION_CAP
The client's task-notification cap, the one coupling point to the client: a
client-cap change is a one-line edit. A rendered event over it spills to a file,
its pointer `{spilled_event_path, event_bytes}` riding in its place.

## enum ChannelPhase
Read at send time, never captured. A job outliving its eval keeps its decl and
everything it closed over, so a spawn-time snapshot would let work started
before verification emit to an unproven peer - exactly what the gating forbids.

Channels are optional and start lazily on the first `interface channel open`,
which is why `Closed` is an ordinary state and an emit there is a plain
catchable error rather than a fault.

## enum ChannelMode
User's two modes, both required to be named at the open with no default:
`async` for a harness whose agent holds a Monitor on a websocket, `sync` for one
with no such mechanism, its agent taking the packets by calling `interface
channel sync`. The phases, the handshake, the gate, and the refusals are the
same in both; only where a packet waits differs.

## struct ChannelEpoch
Each open begins an epoch and its close ends it, so a packet or a claim of an
earlier open's never reaches a later one, and a reopen never inherits a
verified or claimed state.
