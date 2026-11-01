# controller.rs

The worker's controller, the one its process owns, as the MCP's is a
controller over its own systems (User): it starts the engine's system and the
operator's, routes their asks and emergencies, and stops them in order. The
Channel and the peer links are the MCP's, so what an eval asks of them goes up
the session; a pin binds the worker's own operator, so it is held here.

## fn log_status, fn log_closed
As the MCP's controller logs its systems: a status change, and an end no stop
asked for, both unsaid while a shutdown runs.

## struct Supervision
The operator and the observer that feeds it, stopped together.

## struct GrammarSshController
### fn start
The engine's system first, its asks routed through the worker's route, then
the operator on the namespace's homes and its observer. The worker is never
the namespace's test host, so the operator's sampling half stays off.

### fn engine
A clone of the engine's handle for a request's task, or the failure a stopped
engine answers, so a request arriving during the shutdown is refused rather
than blocked.

### fn teardown
Every call in flight cancelled and its process tree reaped, the session's end's
first step, before its drain.

### fn shutdown
The observer stops first, so nothing observes a stopping engine; then the
engine's teardown, the operator, and the engine's system once no request holds
it, bounded by `ENGINE_RELEASE_GRACE`, as the MCP's controller stops its own.

## struct Asker
The session's way to its MCP, attached while a session runs. Each ask takes the
next id and a waiter, the frame queued under the same lock that checks a
session is attached, so an ask never waits on a session that has gone.

### fn detach
The session's end lets every waiting ask go, each refused: the eval that asked
learns its MCP is gone rather than waiting out the deadline.

### fn ask
An ask unanswered within `ASK_DEADLINE` is refused, so an eval is never held
on an MCP that stopped answering.

### fn emergency
An emergency goes up the session when one is attached; with none, the log's
line is its record.

## struct WorkerRoute
### fn answer
A pin is held on the worker, the operator's live layer its watchdog reads. The
config is the MCP's, its Channel's settings live there, with the worker's own
supervisor lines in place of the MCP's, since those govern the worker's
operator. A send and a stop go up as they are; a remote send is prepared here
first, its files read where they are, and goes up inline.

## fn with_supervisor
The record's `supervisor` table replaced, its shape the MCP's, so a
definition reads one shape on any host.

## struct WorkerWatch
The operator's emergencies go to the session's MCP, whose policy reports them
to the agent, and to the worker's log.

## fn engine_health, fn spawn_observer
Each interval, the engine's health and whether a session listens, as the MCP's
observer passes its own.
