# session.rs

The MCP's end of one session: its hello and the worker's reply, then its
requests answered by id, the worker's asks and emergencies handed to the
session's owner, a ping each interval, and an end every waiter learns of.

## enum SessionEvent
What the owner reads beside its responses, on a channel of its own: an ask to
answer, an emergency, and the session's end, once. An owner that stops reading
holds nothing up, the channel being unbounded and each event small.

## struct Responder
### fn answer
An ask's way back, by value, so it answers once. An answer after the session's
end goes nowhere, the eval that asked having been told already.

## enum Out
What the writer takes: a frame, the close's frame as the stream's last, or a
stop with nothing more written.

## struct Outbox
A frame is encoded and held to the session's ceiling as it queues, so a request
too large fails itself, never the session, and the writer sends bytes only.

### fn last
The close queues as the last frame, so everything queued before it is written
first and nothing after it is.

### fn stop
The reader's end stops the writer, which would otherwise wait for a frame
that never comes.

## struct Shared
The requests in flight and the session's end, under one lock each. The end
lets every waiter go by dropping its sender, so a request in flight answers
`remote::engine::session_lost` with the end's reason, its outcome unknown,
and nothing resubmits it.

### fn end
The first reason stands; a later end, the writer's and the reader's both
seeing one loss, changes nothing.

## struct Session
### fn open
The hello is sent and the reply read within `REPLY_DEADLINE`, a worker's start
among it, then both directions take the negotiated ceiling. The writer, the
reader, and the pinger run as tasks of their own.

### fn request
A request's id is the next in the session's order, registered before its frame
queues, so its response can't arrive first. A session that has ended refuses
it at once.

### fn close
The close queued as the last frame, unless the session has ended already, and
the session ended, every waiter let go.

### fn finish
The close, then the writer awaited within its deadline, so the close is
written and the stream ended before the transport under it goes.

### fn drop
The drop closes the session and stops the reader and the pinger; the writer
outlives them long enough to write the close and end the stream.

## fn write_loop
Frames go in queue order until the last, then the stream ends; a stop ends it
with nothing more, and a failed write ends the session.

## fn read_loop
Each frame finds its waiter or its owner; a response for an id nothing waits
on, a request the MCP stopped waiting for, is dropped. No frame within
`SESSION_IDLE`, the stream's end, a frame that doesn't decode within the
session's budget, or the worker's close ends the session and stops the
writer.

## fn ping_loop
A ping each `PING_INTERVAL`, so the worker's idle window never lapses on a
session that is merely quiet.
