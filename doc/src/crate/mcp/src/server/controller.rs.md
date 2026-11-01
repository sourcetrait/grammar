# controller.rs

mcp's controller, the one the process owns: it starts the systems it serves
on, routes between them, and stops them in order. It holds the face, which
starts and routes the Channel's and the peer links' systems
(`server/channel/face.rs`), and starts the engine's system, the remote
engines', and the operator's when it supervises. A system never calls
another: each handler it hands a system holds what that system's routes need,
the engine's `HostRoute` the face's two systems (`server/host.rs`), the
operator's `OperatorWatch` the emergency policy (`server/operator.rs`), and
the remote engines' `RemoteRoute` their workers' asks (`server/remote.rs`).

## fn log_status
A system's status change is logged from its handler, on the system's control
task, and a shutdown's own changes go unsaid.

## fn log_closed
subsys calls `on_closed` only for an end its control did not ask for, so this
names an end nobody asked for; one while a shutdown runs is the shutdown's own.

## struct Supervision
The serve path's: the operator and the observer, which feeds it the engine's
health and the Channel's phase each interval.

## struct McpController

### field config
The runtime config the process resolved once at its start (`run.rs`), which
every system starts on and every reader reaches through the controller: the
face's hub, the engine's host asks, the tools' instructions, and
`interface info`.

### field engine
Behind a lock, so the shutdown can take it while a tool's request holds a
clone: a request takes its own clone first, and one arriving after the
shutdown took the engine answers the stopped engine's failure.

### field remote
The remote engines' system, behind a lock as the engine's is, so a request
arriving after the shutdown took it answers `daemon::unavailable`.

### field lock
The namespace's lock its entry took before the engine (`run.rs`), held for the
systems' life and released by the stop once the engine and the operator have
stopped, so no other holder's engine meets this one's. The in-process harness
starts controllers with none, its tests sharing one namespace a process.

### field stopping
Set as the shutdown starts. The systems' handlers read it, so the ends the
shutdown asks for go unsaid.

### field stopped
An async lock held across the whole shutdown, so a second caller, the signal
sweep beside the ordinary end, waits for the first's to finish rather than
returning while the systems still stop.

### field sweep
The signal sweep's task, which the ordinary end aborts before it stops the
systems, so a late signal cannot run a second shutdown and exit the process
under it. The sweep takes itself out of the slot when its signal comes, so the
ordinary end never aborts a sweep that has begun stopping.

### fn start
The face's systems first, then the engine's, whose host asks reach the face's,
then the remote engines'. `supervise` is the serve path's: the operator, its
observer, and the policy's hold on the Channel's spam pair. The one-shot CLI
and the in-process harness start unsupervised.

### fn start_engine
The policy is made before the engine, and a supervised start puts it in the
face's hub slot before any system it answers runs. The engine's handler is the
route: the face's Channel and links, the config, and the shutdown flag. The
remote engines' handler takes the same route and the same policy, which this
returns, so a worker's asks and emergencies take the paths a local eval's and
the operator's do.

### fn start_remote
The system starts on the config's engines whether it names any or not, so
every controller holds one and stops it the same way; no session opens until a
tool names an engine.

### fn request
The tools' requests go to the engine directly, each on its caller's own task,
so a long eval holds no other request up; the engine's system answers each
from a task of its own.

### fn remote
A request of a remote engine, on its caller's task as the engine's are.

### fn sweep_signals
The sweep's shutdown is the controller's stop, after which the sweep removes
the scratch directories and exits (`server/channel/face.rs`).

### fn stop
The order is the shutdown's point. The remote sessions close first, so every
worker's work ends with its session as the MCP goes; then the Channel's peer
is told that the host is going away (`ChannelCloser::close_for_shutdown`);
then the observer stops, so nothing observes a stopping engine; the engine's
`Teardown` cancels the calls in flight and reaps their trees; the operator
stops next; the engine's system stops once no request holds it; and the
namespace's lock goes last, as late as anything that still works on the
namespace.

### fn stop_remote
Every session is closed before the system stops, so a request in flight lets
the system go at once, answered `daemon::session_lost`.

The Channel's and the peer links' systems end with the process: their handles
are shared by the routes, the hubs, and the policy, whose tasks live until
then, and the Channel's peer has had its close already.

## const ENGINE_RELEASE_GRACE
After the teardown cancels the calls in flight, their requests let go of the
engine within milliseconds; one whose eval is hung where nushell never checks
never does, so the wait is bounded and the engine is left to the process's
exit.
