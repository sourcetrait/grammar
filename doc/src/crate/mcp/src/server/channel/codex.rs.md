# codex.rs

The Channel's async face for a Codex client. Codex reads no wss stream, so the
MCP dials out instead: it finds the app-server through its daemon, connects to
its control socket, and delivers each line the Channel sends up into the
calling thread as a `turn/start`, the harness doing nothing. The face stands
where a wss hub would, a `Hub` in the face's slot with the same lanes, so the
Channel's system, its gate, its claim, and its verify window serve it
unchanged. Unix only: the control socket is a Unix socket, and the vendor
crate's client is Unix's.

## const APP_SERVER_WAIT
One bound for the handshake, `initialize`, and each delivery: the app-server
answers a `turn/start` with its turn in progress rather than at the turn's end,
so a delivery that takes longer is a stuck session, never a slow model.

## struct CodexSession
What the open starts and the delivery task then owns: the initialized client and
the receiving ends of the hub's two lanes. It exists apart from the `Hub`
because its claim has to wait for the hub's install: a greeting sent before the
slot holds the hub would be dropped as a line for no open.

## fn start
The daemon is asked first, `codex app-server daemon version` naming the socket
(`vendor`'s `codex`), and the connect and `initialize` run after, each on the
blocking pool since the client is synchronous. Either failing refuses the open
`channel::start` with the step's own words, the socket's path among them, and
the caller closes the Channel it opened, as for a wss hub that fails to bind.
The hub has no shutdown sender: no accept loop waits on one, and a dropped hub
drops the close lane, which ends the delivery task just as well.

## fn CodexSession::serve
The claim is the session itself: once the hub is installed the face tells the
Channel it is claimed and sends the greeting through it, as the wss hub does on
its first connection, so the agent sees `mcp/channel/Open` in its thread and
answers `interface channel ready` there.

## fn CodexSession::deliver
One line at a time in the Channel's order, each answered before the next, the
close lane first: a close stands the session down and acks, a refused delivery
or a lost session is the peer's end, and either way the lane closes before the
Channel hears `PeerGone`, so an `existing` open reads the end from the lane at
once. The session's close has nothing to carry the Channel's frame: Codex keeps
no Monitor, and the app-server sends no close back, ending its side as it does
for every client.

## fn deliver_one
The thread is read at each delivery, not at the open, so an `interface info`
from another thread moves the Channel to it from the next line on. The packet
rides as the tool output's text, namespaced `grammar` and named `channel`, which
the model reads as `grammar.channel`, clear of every MCP tool's own output.
