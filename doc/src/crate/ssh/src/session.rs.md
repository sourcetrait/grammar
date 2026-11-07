# session.rs

grammar-ssh's one session, on its stdin and stdout, which a `remote engine
connect` holds for as long as it persists (User). The end of its stdin, the
MCP's close, an idle window with no frame, or a stop signal, a console event on
Windows, ends the session, cancels its calls, and drains them for
`DRAIN_DEADLINE`; the controller's shutdown then removes the scratch and the
cache home and releases the lock (`controller.rs`).

## enum SessionEnd
How the session ended, logged as grammar-ssh stops; every end is an ordinary
one, so it exits 0.

## enum Unsent
## struct Outbox
A frame is encoded once and held to the session's ceiling and its decode
budget before it queues, the budget by decoding it as the MCP will. A response
past either is refused in its place under its own id, so one oversized result
never breaks the MCP's decode and with it every call on the session.

## struct Asker
The session's way to its MCP, attached while the session runs, as the MCP's
face is its way to the agent's Channel. Each ask takes the next id and a
waiter, the frame queued under the same lock that checks a session is
attached, so an ask never waits on a session that has gone.

### fn detach
The session's end lets every waiting ask go, each refused: the eval that asked
learns its MCP is gone rather than waiting out the deadline.

### fn ask
An ask unanswered within `ASK_DEADLINE` is refused, so an eval is never held
on an MCP that stopped answering.

### fn emergency
An emergency goes up the session when one is attached; with none, the log's
line is its record (`emergency.rs`).

## fn read_hello
The MCP's hello within `HELLO_DEADLINE`; one that doesn't decode is refused on
the way, so the MCP learns why.

## fn refuse
A start that failed reads the hello and answers its refusal.

## fn serve
The hello answered, then both directions take the negotiated ceiling. The
asker is attached for the session's life, so an eval's asks reach this MCP and
no other. An idle end and a signal end send a close naming why; a stream that
ended or broke has no one to tell.

## fn write_loop
Frames go in queue order; the writer ends once nothing can queue, the session
over and every request's task gone.

## struct Stops
On Unix, SIGTERM, SIGINT, and SIGHUP end the session as its end does, so a
grammar-ssh a user or a service stops leaves no call running.

On Windows the console's events stand in for them: `CTRL_BREAK` as TERM,
`CTRL_C` as INT, and `CTRL_CLOSE` as HUP. Once taken, break and Ctrl-C no
longer end the process at once, as the console's default handler would. After
a close event the system ends the process within the console's close time-out,
five seconds by default, while an end may spend `DRAIN_DEADLINE` draining its
calls and as long again on its writer, so a close can cut the end off before
the shutdown removes the scratch and releases the lock. A process without a
console receives none of these events, and its session ends by its stream, the
MCP's close, or the idle window.

## fn serve_frames
Requests run each on a task of their own, an eval in one slot of `calls` and
every other request in one of `control`, so a session full of evals still
answers an await list or an await stop; a request past its slots is refused
`Busy` at once rather than queued. A request whose id is in flight ends the
session, since its response could not tell the two apart. An answer to an ask
goes to the asker; a ping is ponged. The session's view and its interface parser
are made at its start, so a view on grammar-ssh's engine dies with the session.

The end detaches the asker first, so an eval asking now is refused at once,
then tears the calls down and drains them; a call that outlasts the drain is
dropped. The calls' cache entries stay for the rest of grammar-ssh's life, a
rerun's definition among them, and go with the cache home at its end.

## fn send_response
A response sent once; one refused for its size is replaced by the refusal.

## fn respond
Each tool call runs here, refused `Unavailable` when the engine has stopped. An
eval's request runs as local: the host the MCP named is its own alias,
meaningless to grammar-ssh, so the request's host is grammar-ssh's own engine.
An interface call's subcommand is parsed here and run on this engine with the
session's view (`crate/engine`'s `desk.rs`), so the MCP sends a command it knows
only as text.

An await call is answered as the other calls are; its job's events ride the
session's asks after the answer. An interface call runs with its request's
nonce, so an `await rig` it carries starts its job under the call's own nonce,
the `await` its events name.

## fn value
A call's answer, its value or its rows; a request the engine's system fails to
take answers as the call's failure, so the MCP reads one shape whatever failed.

## struct SessionDesk
What a session lends an interface call: grammar-ssh's engine, its identity, and
the session's view, made fresh at the session's start. Its interface block
leaves nothing out, since the MCP's deny set refused a denied command before it
crossed.
