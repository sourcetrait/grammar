# face.rs

The Channel's and the remote channels' commands and the Channel's shutdown,
shared by the client and the test driver, so both entries serve them from one
piece of code. The face is the controller of two systems, the Channel's
(`crate/channel`) and the peer links' (`crate/peer`): it starts and holds both,
binds each async open's hub, keeps the open's verify timer, pumps the Channel's
lines into the hub, and hands the links' relays and reports to the Channel.

## struct ChannelFace
What a server needs of the Channel and the remote channels: their two systems,
the open's hub, the lock that serializes an open, the config each open binds
by, and the identity its greeting carries. mcp's controller starts and holds it
(`server/controller.rs`), and both servers serve every Channel command and every
remote channel command through it (`server/client.rs`,
`server/tool/interface.rs`).

### field config
The process's config, which the controller hands the face: each open's hub binds
its port and presents its identity (`hub.rs`).

### field channel
The system this face started. An eval's send reaches the same one through the
engine's handler (`server/host.rs`'s `HostRoute`), so what a body emits on is
what the face serves; the observer reads its phase (`server/operator.rs`).

### field peer
The peer links' system this face started. A body's remote send reaches it
through the engine's handler (`server/host.rs`'s `HostRoute`), which holds it
beside the Channel's.

### field channel_open_lock
Async because it is held across the open's requests and its hub's bind.

## fn ChannelFace::start
The Channel's system starts first, on the config's thresholds, its handler the
face's hub pump; the peer links' after it, their handler holding the Channel,
so a link's first relay has a Channel to reach.

## fn peer_params
The links take the config's values as plain strings, every path expanded: the
key pair as the config resolved it, the remotes file the config's sibling, and
the stand-ins a certificate path's unset variables expand to (`config.rs`'s
`expand_defaults`), so the peer crate holds no copy of mcp's home model. The
agent's config file is named for the refusal a host without `[identity]` gives.

## struct PeerPump
The face's handler of its peer links. A relay goes onto the Channel through its
gate, as a body's send would, its files already in the inbox; a report is
emitted from `mcp`, best effort. The links' control task awaits each before it
takes the next, so the Channel takes them in the order the links sent them up.

## fn relay_onto
A relay the Channel refuses is logged by its delivery's id, the one record of
a peer's message that reached no agent.

## struct Hub
One async open's hub as the face holds it: its epoch, its endpoint, the queue
its peer task writes to the socket, the close lane, and the shutdown sender,
whose drop ends the accept loop.

## struct HubSlot
The face's side of the open, shared with its pump and its timer: the hub, the
armed verify timer, the emergency policy, the epochs the face mints, and the
highest epoch closed.

### field closed_through
Set by every close, under the hub's lock, before it looks for the hub to
signal: a close can come while its open's hub is still starting, and the hub
installed after it would serve an open already closed. A shutdown's close marks
every epoch minted so far.

### fn install
Refuses a hub whose epoch has closed, under the same lock the close takes, so
the check and the close never interleave; the caller drops the refused hub,
which ends its accept loop (`server/tool/channel_open.rs`).

### fn next_epoch
From 1, so the Channel's closed epoch, 0, is never an open's.

### fn peer_gone
The peer task drops its queue's receiver when it ends, so a closed queue is the
peer gone, read the moment it happens, before the system hears the notice.

### fn deliver
A line for another open's epoch is dropped: the hub it was for is gone.

### fn close
The close signal does not share the packet queue. A close queued behind traffic
could arrive late or not at all, and without an explicit close frame the agent
sees a bare 1006 - indistinguishable from a crash. Dropping the hub drops its
shutdown sender, which ends its accept loop, and the open's verify timer stands
down with it.

### fn close_now
The completion half exists for shutdown. The frame is written by the hub task
asynchronously, so a caller that exits the process immediately after asking for
a close races that write and leaves the peer with exactly the bare 1006 the
explicit close exists to prevent. Awaiting an ack makes the ordering
deterministic instead of a sleep long enough to usually work.

### fn arm_verify
Holding the sender is what keeps the timer armed and dropping it is the cancel,
so verification, a close and a re-arm all stand the timer down. That is also
why re-opening cannot leave two timers racing one channel. The epoch rides with
the sender, so standing one open's timer down leaves another's armed.

### fn install_policy
Supervision's, at the controller's start (`server/controller.rs`): until a
policy is installed, the Channel's spam pair goes unanswered, as on a host that
starts no operator.

### fn raise
Each emergency is answered on a task of its own. The pump raises it on the
Channel's control task, and the policy's announcement is a request to that same
system, whose answer only that control task delivers, so awaiting it there
would wait on itself.

## struct HubPump
The face's handler of its Channel's system. It does its work before it returns,
on the control task, so the system's next answer reaches its waiter only after
the line is queued or the frame signalled. An emergency is the one it hands on,
to the policy's own task (`raise`).

## const CLOSE_FLUSH_GRACE
Bounded because a peer that has already gone must never hold the shutdown open.

## fn ChannelCloser::close_for_shutdown
Without this a shutting-down host simply vanishes and the agent sees a bare
1006, indistinguishable from a crashed host - the very distinction the explicit
close frame exists to provide.

The hub's frame is signalled first, then the logical Channel closes, so the
system's close finds no hub left to signal again; the ack is awaited last. The
frame is written by the hub task, so the wait is an ack rather than a sleep: the
hub fires the completion after its flush. A sleep long enough to usually work
would be a race dressed as a guarantee.

The Channel's close is awaited for the same grace. Its system takes one message
at a time, a send's file writes among them, so a stalled write would otherwise
hold the shutdown, and the engine's cleanup behind it, past any bound.

## fn spawn_signal_sweep
Exiting is part of the contract: handling a termination signal without
terminating would make the host ignore the very thing it was told to do.

It is also what makes the handler observable. A swept host reports
`code() == Some(128 + signo)`, while a host that merely took the signal's
default action reports `code() == None` and `signal() == Some(signo)`. Only the
former proves the sweep ran, which is what the system test measures.

The shutdown is the controller's stop, which closes the Channel first and then
stops the engine's side, if any (`server/controller.rs`). The signal's number is
tokio's raw value, so the portable frontend takes no nix. The sweep's task is
handed back, so the controller that owns it can stand it down at an ordinary
end.

The scratch directories go last, after the components that write into them have
stopped: `process::exit` never returns to `serve_or_oneshot`, whose ordinary end
would remove them otherwise. The sweep is Unix's; elsewhere none is installed,
and the ordinary end does its work.
