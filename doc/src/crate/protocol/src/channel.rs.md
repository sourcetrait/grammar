# channel.rs

The Channel's portable data: the reservations and caps every side enforces
alike, its modes, phases, and state, and a send as data. The Channel is the
MCP's, and a remote grammar-ssh's send reaches it as an ask (session.rs).
Closures and borrowed nushell values never cross: a send crosses as data and its
eval's stop stays on the eval's side.

## const MCP_RESERVED_PREFIX / const REMOTE_RESERVED_PREFIX / fn reserved_prefix
Every model the host stamps lives beneath one of the three: `mcp/` for the local
host's own packets, `remote/` for those about a remote, whose peer need not be
an MCP (User), and `channel/` for the host's messages between agents, a chat
(User). The value of a reservation is that it makes provenance a mechanical
check: anything claiming a prefix from a source that is not the host can be
rejected without interpreting it. Enforced wherever a body picks a model, a
definition's send local or remote, through `reserved_prefix`.

## const CHANNEL_RESERVED_PREFIX / fn relay_reserved_prefix
A peer's relay is held to `mcp/` and `remote/` alone, the receiving host's own:
a `channel/` model reaches it from a peer's host, which held its own bodies to
the reservation and is trusted by its known key, so a chat passes the relay.

## const CHAT_MODEL / const CHAT_MESSAGE_FILE / const CHAT_SUMMARY_MAX_CHARS
The chat's model, the file every chat attaches, and its summary's bound, which
keeps the summary on the line however long the message.

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
client-cap change is a one-line edit. A rendered line over it spills its
`attached`, then its `data`, each to its file's size (`crate/channel`'s
`render.rs`).

## const MODEL_MAX_CHARS / fn model_too_long
The one field a spilled line keeps that its sender chooses, bounded so a spill
always fits the cap. Held wherever the reservation is: a definition's send, a
remote send on the sender's side, and a peer's relay.

## enum ChannelPhase
Read at send time, never captured. A job outliving its eval keeps its decl and
everything it closed over, so a spawn-time snapshot would let work started
before verification emit to an unproven peer - exactly what the gating forbids.

Channels are optional and start lazily on the first `interface channel open`,
which is why `Closed` is an ordinary state and an emit there is a plain
catchable error rather than a fault.

## enum ChannelMode
User's two modes, which the open chooses for the caller's harness, never the
agent: `async` for a harness that takes packets as they come, a Monitor on the
hub's websocket or Codex's thread, `sync` for one with no such mechanism or one
the host doesn't detect, its agent taking the packets by calling `interface
channel sync`. The phases, the handshake, the gate, and the refusals are the
same in both; only where a packet waits differs.

## struct ChannelEpoch
Each open begins an epoch and its close ends it, so a packet or a claim of an
earlier open's never reaches a later one, and a reopen never inherits a
verified or claimed state.
