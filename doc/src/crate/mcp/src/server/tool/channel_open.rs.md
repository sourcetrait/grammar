# channel_open.rs

## const VERIFY_WINDOW
A code constant, not a config option: it is a property of the handshake rather
than of a deployment.

## const CLOSE_UNVERIFIED
A distinct code because close codes and reasons reach the agent verbatim, so the
three teardowns - refused 1013, planned 1000, unverified 1008 - stay tellable
apart.

## const CLOSE_UNSTARTED
A hub that fails to bind closes the open it was for; no peer hears it, so the
code is for the record.

## fn resend_open_packet
A gone peer is the only way to learn the connection has gone: the peer task's
lane closed, read here first, or the peer's end the Channel's system already
heard. Nothing else reports a peer that simply went away, which is why an
`existing` open re-greets rather than trusting its own state.

## fn arm_verify_timer
Expiry asks the Channel to close the open of its epoch while still unverified,
which sends the hub its frame; verification or a close drops the sender, which
stands this task down instead. That is why cancellation is a drop rather than a
message - it cannot be missed.

## fn start_open
The Channel opens before the hub binds, so the hub's claim can never reach the
system ahead of the open it belongs to. A hub that fails to start closes the
open again, the inbox taken back for the caller to remove.

A close can land while the hub starts, the agent's own `interface channel
close` or an early send's teardown, neither of which takes the open's lock.
Its frame finds no hub to signal, so the face's hub slot remembers the epoch
closed and refuses the late hub (`face.rs`'s `HubSlot::install`): the hub is
dropped, which ends its accept loop, the peer links are told the inbox is gone,
and the open answers that the Channel closed while its hub started.

## const CLOSED_DURING_START
The open's answer when a close won the race with its hub's start.

## fn tell_inbox
The peer links land a peer's files in the Channel's inbox, which only the
Channel's state holds, so the face tells them each time that state changes: an
open's inbox the moment the open is in, and none when a close takes it, the
hub's failed start as a deliberate close. It waits for the links' answer, so
what follows runs on the inbox they hold.

## struct ChannelOpenResult
`wss` is null for a sync channel rather than absent, so the result keeps one
shape in both modes and `mode` says which it is.

## fn channel_open
The decide-then-start sequence spans an await, so it runs under one guard: two
concurrent calls would otherwise both read Closed and both bind a hub.

## fn open_or_report
A new open makes its inbox first (`scratch.rs`'s `make_channel_dir`) and hands
the path to the channel, so the emit path writes an attachment without
composing it again, and the result reports it, so the agent learns where a
spill lands from the same call that gave it the endpoint. A start that fails
takes the inbox back and removes it. An `existing` open answers the inbox its
Channel opened with.

A mode is never switched under a live channel: an open in the other mode is
refused `channel::wrong_mode`, naming the open one, since switching would strand
the packets queued for the first. The agent closes and opens again.

`existing` is the normal case, not a fault, and reading it as one is expensive.
The host outlives an agent's context - it keeps running across a `/clear`, and
so does a persistent Monitor - so a channel the agent has no memory of opening
is usually still open and still claimed. The re-send is the liveness test: a
call that returns has already proven the peer, and only `channel::peer_gone`
says otherwise. So `peer_gone` is the only signal that calls for
close-then-reopen; tearing down on `existing` destroys a working channel and the
Monitor draining it. The handshake exists because the agent loses its memory,
not its connection.

The re-send is guarded on `claimed` because an unclaimed hub has no connection
to test and greets the next Monitor itself - an extra packet there would only
queue a duplicate. A sync channel unclaimed still holds its first greeting,
untaken, for the same reason.

The timer is armed on both paths: an existing channel has to re-prove its peer
too, and a fresh arm stands the previous timer down.
