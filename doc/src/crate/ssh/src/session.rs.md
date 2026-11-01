# session.rs

The worker's one session, on its stdin and stdout. A worker serves one session
and ends with it: the end of its stdin, the MCP's close, an idle window with no
frame, or a stop signal, a console event on Windows, ends the session, cancels
its calls, drains them for `DRAIN_DEADLINE`, removes the session's call cache
entries, and returns, the runtime's shutdown then removing the scratch and
releasing the lock. Nothing the session made outlives it, as a command's work
ends with its SSH session (User).

## enum SessionEnd
How the session ended, logged as the worker stops; every end is an ordinary
one, so the worker exits 0.

## enum Unsent
## struct Outbox
A frame is encoded once and held to the session's ceiling and its decode
budget before it queues, the budget by decoding it as the MCP will. A response
past either is refused in its place under its own id, so one oversized result
never breaks the MCP's decode and with it every call on the session.

## fn read_hello
The MCP's hello within `HELLO_DEADLINE`; one that doesn't decode is refused on
the way, so the MCP learns why.

## fn refuse
A worker that couldn't start reads the hello and answers its refusal.

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
worker a user or a service stops leaves no call running.

On Windows the console's events stand in for them: `CTRL_BREAK` as TERM,
`CTRL_C` as INT, and `CTRL_CLOSE` as HUP. Once taken, break and Ctrl-C no
longer end the process at once, as the console's default handler would. After
a close event the system ends the process within the console's close time-out,
five seconds by default, while an end may spend `DRAIN_DEADLINE` draining its
calls and as long again on its writer, so a close can cut the end off before
the runtime's shutdown removes the scratch and releases the lock. A process
without a console receives none of these events, and its session ends by its
stream, the MCP's close, or the idle window.

## struct Served
The ids in flight and the calls' nonces. A request whose id is in flight ends
the session, since its response could not tell the two apart; the nonces name
the cache entries the end removes.

## fn serve_frames
Requests run each on a task of their own, an eval in one slot of `calls` and
every other request in one of `control`, so a session full of evals still
answers a kill or a process list; a request past its slots is refused `Busy`
at once rather than queued. An answer to an ask goes to the asker; a ping is
ponged.

The end detaches the asker first, so an eval asking now is refused at once,
then tears the calls down and drains them; a call that outlasts the drain is
dropped. The cache entries go last, once nothing writes to them.

## fn call_nonce
An eval's, a rerun's, or a call's nonce, the name of its cache entries.

## fn send_response
A response sent once; one refused for its size is replaced by the refusal.

## fn respond
Info is the worker's own and needs no engine. Every other request goes to the
engine, refused `Unavailable` when the engine has stopped.

## fn wire
Each request runs as local here: the host the MCP named is its own alias,
meaningless on the worker, so the request's host is the worker's own engine.
A request the engine's system fails to take answers as the call's failure, so
the MCP reads one shape whatever failed.

## fn purview
A purview request's answer, the engine's failure to take it as the runtime's
internal error, as the MCP's own purview requests read it.
