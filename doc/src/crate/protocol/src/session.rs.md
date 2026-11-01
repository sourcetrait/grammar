# session.rs

A session after its hellos: one stream between the MCP and one worker, an SSH
session's stdio, carrying the engine's requests one way and their answers the
other, with the engine's host asks running back the opposite way. The worker
is a remote engine, and what crosses is what the local engine's system carries
between it and its controller: its requests, its purview requests, and its
asks, so the MCP's handlers serve a remote host through the same calls.

Nothing on a session outlives it. A result leaves the worker in its response
and is never kept; the worker removes its calls' cache entries at the
session's end, and its calls end with it, as a command ends with its SSH
session (User).

## const PING_INTERVAL
## const SESSION_IDLE
The MCP pings each 10 s, and either side ends a session that carries no frame
for 30 s, so a lost connection that no read sees end, a peer's machine gone,
ends the worker within the window rather than leaving it running.

## const ASK_DEADLINE
An ask waits 30 s for its answer, so an eval is never held on an MCP that
stopped answering; past it the eval's ask is refused.

## const DRAIN_DEADLINE
A session's end gives its calls 5 s to stop once they are cancelled, then
drops them.

## struct Ping
## struct SessionClose
A close says why where its sender wasn't asked to end the session: the
worker's idle end and its signal end carry their reason.

## enum McpFrame
## enum WorkerFrame
The two directions' frames. Requests and asks each carry an id their sender
draws in order, unique in the session, so a response or an answer finds its
waiter without a table of its own on the wire. A cancel is no frame of its own:
it is the engine's `Kill` under the call's nonce, which the MCP minted, as
`interface process kill` is.

## enum EngineRequest
### fn is_call
An eval, a rerun, or a call takes one of the session's call slots; every other
request takes a control slot, so a session full of evals still answers a kill,
a process list, or an info.

## enum EngineResponse
`Refused` is the worker's refusal before its engine took the request, or an
answer past the frame refused in its place, never the engine's own failure,
which its answer carries.

## struct WorkerInfo
What `interface info` reports of a host that only its worker knows: its nom,
its workdir, its two scratch directories, and its plugins. The signature block
and the purview come from its engine's own requests.

## struct PluginInfo
A positional pair, as the tools have always shown it.

## enum SessionAsk
The engine's host asks a worker can't answer itself, carried to the MCP, whose
Channel and peer links are the agent's. A pin is held on the worker, since it
binds the worker's own operator, so it never crosses. A remote send crosses
prepared: its files read and its event rendered on the worker, where the paths
are, and carried inline within the frame's ceiling, so the MCP reaches no
file of the worker's.

## struct PreparedSend
## struct SendFile
A remote send's files are bytes beside their dests, the shape the peer links
send, each dest held to `safe_dest` on both sides.

## enum HostAnswer
## struct HostStop
The answer to an ask, the same an eval takes from its local host: an id, a
refusal, a spam stop for the eval to take, the config, a pin held, or a stop
noted.
