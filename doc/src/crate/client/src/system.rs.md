# system.rs

The MCP's remote engines as a subsys system under its controller: one session
a configured host, opened at the host's first request and reopened at the
first request after one ends. Each request is answered on a task of its own,
so one waiting on a connect holds no other up. A worker's asks go up to the
controller as requests it answers, and its emergencies as packets.

## const RELEASE_GRACE
How long a stop waits for the requests in flight to let the system go.

## enum ToRemote
## enum FromRemote
A request and its response, the close and its answer, an ask and its answer,
and an emergency, which no request waits on.

## struct RemoteRequest
## struct RemoteResponse
A request names its engine by alias; its response is the worker's, or the
transport's error.

## struct RemoteAsk
## struct RemoteEmergency
A worker's ask and emergency under the alias of its engine.

## struct CloseRequest
## struct Closed
The close, answered once every session has closed.

## struct RemoteSysParams
The configured engines and the bounds each session's hello asks for.

## struct RemoteSysConfig
## struct RemoteSysPaths
The system reads no file, so its config holds nothing and its paths are the
MCP's homes.

## struct RemoteSystem
### fn on_channel_recv
A request runs on a task of its own, its work on a further one, so a panic
answers `unavailable` and the system serves on. The close runs the same way.

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
### fn session
An open session that hasn't ended serves; otherwise one opening is joined or
a new open begins, an ended session it replaces closed on a task of its own.
The first waiter to see the open finish puts the session in its slot, or
clears the slot after a failure, so the next request opens again and nothing
is resubmitted. A session that opens after the close began is closed at once
and its request refused.

### fn close_all
The close stops any session from opening, then closes each one, its requests
in flight answering `daemon::session_lost`.

## fn closing
The refusal a request meets once the close has begun.

## fn open_relayed
A session opened and logged with its worker's pid and nom, its events relayed
for its life.

## fn relay
Each ask answered on a task of its own, each emergency sent up, and the
session's end logged.

## struct RemoteHandle
### fn close
Every session closed, before the stop.

### fn stop
The system stopped once the requests in flight have let it go; one still held
past the grace is left to the process's end.
