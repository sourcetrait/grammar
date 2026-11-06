# controller.rs

grammar-ssh's controller, the one its process owns, as the MCP's is a
controller over its own systems (User): it holds one engine instance and
inherits its nom (User), starts the operator, routes their asks and
emergencies, and manages grammar-ssh's lifetime, so its teardown is the one
place grammar-ssh's files go (User). The Channel and the peer links are the
MCP's, so what an eval asks of them goes up the session; a pin binds
grammar-ssh's own operator, so it is held here (`host.rs`).

## const LOCK_HOLDER
What the namespace lock's record names a holder, beside the MCP's own: a reader
of the lock's file tells the two programs apart by it.

## fn log_status, fn log_closed
As the MCP's controller logs its systems: a status change, and an end no stop
asked for, both unsaid while a shutdown runs.

## fn held
A slot's lock taken as it stands even when poisoned, shared with the session's
asker (`session.rs`).

## fn record_lock
The record names this process, its pid, its start, and its boot, and its
engine's nom, the one its hello and `interface info` report. It is written once
the engine has started, the lock held since before anything touched the
namespace (`run.rs`).

## fn remove_cache_home
The one difference from the MCP's file persistence: grammar-ssh's end removes
its cache home, its call logs, its definitions, and its emergency logs, for
security, where the MCP keeps its own (User). The namespace's cache home is
checked first, a link or a file in its place refused rather than followed;
`remove_dir_all` removes links within it without following them. The lock is
still held, so no other host is writing into it.

## struct Supervision
The operator and the observer that feeds it, stopped together.

## struct GrammarSshController
### field identity
The engine instance's identity, which grammar-ssh inherits: the nom its lock's
record, its hello, its info, and its emergency log carry. An engine started
again after a failure would be a new instance under a new nom, and its record
written again; the controller lives on through it, so its teardown alone
removes the cache and the logs (User).

### fn start
The one engine first, so its nom is known before anything records it; then the
lock's record, the policy's log under that nom, and the operator on the
namespace's homes with its observer (`supervise`). grammar-ssh is never the
namespace's test host, so the operator's sampling half stays off. A failure
after the engine runs shuts everything down, the scratch and the cache home
removed and the lock released, as the end does.

### fn engine
A clone of the engine's handle for a request's task, or the failure a stopped
engine answers, so a request arriving during the shutdown is refused rather
than blocked.

### fn profile, fn info
What a hello's reply and `interface info` report of grammar-ssh: its namespace,
its uid and pid for the MCP's own log, and its engine's nom; its workdir, its
scratch directories, an agent's scratch for its id beside its transient one, and
its plugins.

### fn teardown
Every call in flight cancelled and its process tree reaped, the session's end's
first step, before its drain.

### fn shutdown
The observer stops first, so nothing observes a stopping engine; then the
engine's teardown, the operator, and the engine's system once no request holds
it, as the MCP's controller stops its own; then the scratch and the cache home
are removed, and the lock goes last, so a successor takes the namespace only
once grammar-ssh is done with it. Every step runs whatever the one before
returned, the first failure reported.

### fn stop_engine
Bounded by `ENGINE_RELEASE_GRACE`, as the MCP's is.
