# system.rs

The MCP's remote engines as a subsys system under its controller: one session
an alias, opened on the entry its first request or a connect brings, and
reopened at the first request after one ends. The system holds no entry: the
controller reads the remotes file at each use and sends the entry with the
request, so a changed entry takes effect at the next session it opens. Each
request is answered on a task of its own, so one waiting on a connect holds no
other up. A worker's asks go up to the controller as requests it answers, and
its emergencies as packets.

## const RELEASE_GRACE
How long a stop waits for the requests in flight to let the system go.

## enum ToRemote
## enum FromRemote
A request and its response, the close and its answer, an ask and its answer,
the states, a connect, and one alias's close with theirs, and an emergency,
which no request waits on.

## struct RemoteRequest
A request names its engine by its endpoint, the entry as the remotes file held
it when the controller read it.

## enum SessionPhase
## struct SessionState
An alias's session as `interface remote engine list` reports it: closed,
opening, or open with its worker's nom and pid, and the last open's failure,
its kind first, which stands until an open succeeds.

## struct StatesRequest
## struct SessionStates
Every alias with a session or a failure to report, by alias.

## struct ConnectRequest
## struct ConnectAnswer
A session opened now on an endpoint where none is open, answered with its
state or the open's failure.

## struct CloseSessionRequest
## struct SessionClosed
One alias's session closed, answered with whether one was open.

## struct RemoteResponse
The worker's response, or the transport's error.

## struct RemoteAsk
## struct RemoteEmergency
A worker's ask and emergency under the alias of its engine.

## struct CloseRequest
## struct Closed
The close, answered once every session has closed.

## struct RemoteSysParams
The bounds each session's hello asks for.

## struct RemoteSysConfig
## struct RemoteSysPaths
The system reads no file, so its config holds nothing and its paths are the
MCP's homes.

## struct RemoteSystem
### fn on_channel_recv
A request, a connect, and an alias's close each run on a task of its own,
their work on a further one, so a panic answers `unavailable` and the system
serves on. The states are read in place, under the slots' lock alone. The
close of every session runs as a request does.

### fn on_stop
Every session is closed.

## fn respond
An answer sent up as its request's response.

## struct Uplink
### fn ask
A worker's ask awaited from the controller, an unanswered one refused with
why, so the eval that asked is never left waiting past its deadline.

### fn emergency
A worker's emergency sent up as a packet.

## type Opening
## enum Slot
A host's session while it opens, its outcome shared by every request that
arrives meanwhile, then open.

## struct Hosts
The slots by alias, and each alias's last open's failure, which the next
success clears.

### fn connect
The session a request would take, opened now where none is open.

### fn close_session
The alias's slot taken: an open session closed, an opening one awaited, then
closed. The answer is whether a live session closed, so a close of an alias
with none says so.

### fn states
Each alias with a slot or a failure, an ended session read as closed.

### fn session
An open session that hasn't ended serves; otherwise one opening is joined or
a new open begins, an ended session it replaces closed on a task of its own.
The first waiter to see the open finish puts the session in its slot, or
clears the slot after a failure, so the next request opens again and nothing
is resubmitted. Each waiter records the open's outcome for the states: a
failure under its kind, a success clearing it. A session that opens after the
close began is closed at once and its request refused.

### fn close_all
The close stops any session from opening, then closes each one, its requests
in flight answering `remote::engine::session_lost`.

## fn open_state
An open session's state, its worker's nom and pid from the hello's reply.

## fn closing
The refusal a request meets once the close has begun.

## fn open_relayed
A session opened and logged with its worker's pid and nom, its events relayed
for its life.

## fn relay
Each ask answered on a task of its own, each emergency sent up, and the
session's end logged.

## struct RemoteHandle
### fn states
None when the system can't answer, so a list reads empty rather than failing.

### fn close
Every session closed, before the stop.

### fn stop
The system stopped once the requests in flight have let it go; one still held
past the grace is left to the process's end.
