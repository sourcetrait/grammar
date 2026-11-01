# system.rs

The engine as a subsys system: a controller starts it, requests through its
handle, answers its host asks, and stops it.

## struct EngineSystem
Its loop only receives. Each request is answered on a task of its own, so a long eval or
a lock wait holds no other request up, and the loop is always free to take a stop.

## struct EngineSysParams
Unit, as `EngineSysConfig` is: the engine's own parameters arrive through `init`,
process-wide, so the system takes none of its own.

## struct EngineSysPaths
The namespace's homes as `init` set them: config, data, and state the data home, cache
and runtime the cache home, and both secrets dirs the keypair's. The engine keeps nothing
outside its namespace.

## fn init
The core takes the system's requester as its host link before any eval runs, so
every eval's asks reach the controller that started this system.

## fn on_channel_recv
The answer is sent from the request's own task, on a clone of the system's sender; the
loop has gone back to receiving by then. A purview request runs on a blocking thread, its
store's file and the equip store's lock being blocking work. A host answer reaching here
answers no waiter, its ask gone, and is dropped: the framework hands every other answer
to its waiter first.

## fn purview_answer
An install adjusts the rows only for an equip with a rig, as the install always did; a
lib alone answers the rows as they stand.

## fn purview_failed
A request whose task failed is answered in its own kind, so its waiter reads the failure
rather than a type it never asked for.

## fn on_stop
A halt tears down every call in flight, as the `Teardown` command does, so a system
stopped mid-eval leaks no process tree.

## fn answer
Every request is answered, a refusal included, so a controller never waits on one the
engine dropped.

## fn contained
A panic in a request's work answers as `internal` on that request rather than ending the
system. An eval's own panic never gets this far, its thread catching it; this is for the
engine's code around the eval, the equip lifecycle's among it.

## struct HostLink
An eval runs on a thread of its own outside the runtime, inside a synchronous nushell
command that waits for its answer, so it asks by the requester's blocking form. The
requester holds the system weakly, so a link outliving its system answers that it is
gone rather than keeping it alive.

## fn EngineHandle::start
The system starts on the namespace `init` set, so `init` comes first, and the equip store
before both (`serve.rs`'s `EngineCore`).

The handler is the controller's. It hears the system's status and its end, an end it
didn't ask for among them, and it answers the host asks (`on_request`). The engine sends
nothing else unasked, so a handler's `on_packet` never sees a packet.

## fn EngineHandle::request
Generic over the request types, each naming its answer, so a controller gets the answer's
own type back: a `ValueAnswer` for an eval, an `EngineResult<ControlAnswer>` for a
command. A request the system could not take or answer is `internal`, the system being
gone.

## fn EngineHandle::control
A command to the local engine, the common case, its delivery's failure folded into the
command's own result.

## fn EngineHandle::stop
A halt, not a drain: the calls in flight are torn down, and the stop returns once the
system and its control task have ended.
