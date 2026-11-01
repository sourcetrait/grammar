# operator.rs

The operator's portable data: what its manager observes of the engine and the
Channel, and the emergencies it classifies. Here so a frontend, a worker, and
the operator share one definition without the operator's Linux monitoring, which
stays in the operator crate. The engine's reading is the engine's own
`EngineHealth`, which the operator names `EngineReading`.

CLASSIFY-FIRST. The watchdog DETECTS and CLASSIFIES resource trouble into an
`Emergency` and sends it to its manager, whose one responder logs and announces it.
Detection is decoupled from response, and internal state is acted on INTERNALLY -
never by relying on a transmit-out succeeding.

RECOVERY IS DEFERRED: no restart, no targeted recovery. The log keeps gathering data
on which conditions actually fire before any recovery is designed. NOTICE is not
deferred, though - a warning that only reaches a log cannot do the job the Warning
family exists for.

Cereal data, so an emergency crosses from the operator to its manager as a packet.
The Channel's spam pair is modelled here beside the supervisor's conditions because
the one responder takes both: the host's Channel classifies its own spam into these
structs. Every count is a fixed-width u64, so an emergency reads the same from any
platform's worker.

## struct Observation
### field engine
None when the engine didn't answer in time. That is an ABSENT reading, not an empty
one: the watchdog keeps its active conditions across it rather than clearing them.

### field listening
The Channel is the host's, so its state reaches the operator as news; a test host
samples only while it is open.

## enum EmergencyKind
### fn name
The log line's `kind`. The Channel's models for each kind are the host's, under its
`mcp/` reservation, so they stay with the responder.

## struct HungEngineThreadEmergency
The pathological residual: an eval whose `Signals` was triggered but which never
stops, because its nu code is stuck where it never polls - a pure-Rust hot loop
inside a builtin, or a bare blocking syscall. Cooperative cancel cannot reach it and
you cannot SIGKILL a thread.

Lane-tagged with a FIELD rather than split into two look-alike variants. A stateless
one also holds a concurrency permit, so enough of them saturate the stateless pool; a
stateful one deadlocks the single serial lane. `pool_held` and `pool_cap` snapshot
that pressure at detection.

An external child or plugin stuck the same way is NOT this condition - killing that
process unblocks the waiting nu thread.

## struct CpuWarningEmergency
THE WARNING FAMILY EXISTS TO GIVE NOTICE BEFORE THE SYSTEM'S OWN ERROR ARRIVES. The
system supplies the error when a level is really exceeded - VRAM ends in an OOM, CPU
simply slows until someone notices, a full filesystem returns ENOSPC. These are the
notice that comes first, so the agent can act while it still has room.

HEAVY LOAD IS NORMAL and how to react is the AGENT's call: the host never reacts to
any of them. That is the whole difference between this family and `Critical` or the
spam error, which are for things we know must not happen.

CPU here can exceed 100 on a multi-core box, since it sums all threads. It is
ambiguous alone - a legitimate heavy transform looks identical to a runaway - which
is why the suffix says Warning and nothing triggers on it.

## struct VramWarningEmergency
Attribution to our own eval children is a later refinement; this reads the whole
card.

## struct ChannelSpamWarningEmergency
NOT the Warning family despite the shared suffix. Spam implies a BUG rather than
load, so it is the one thing we stop.

Fired ONCE per origin, so the report about spam never becomes spam itself. It carries
no payload example deliberately - the agent is already being spammed by that, and can
investigate the cause itself.

`origin`, NEVER `from`. The packet envelope already carries a `from` - the SENDER,
which for anything here is the host - so an event field of the same name would put
two different meanings under one word in a single record. This is invisible in
`emergency.nuonl`, which has no envelope, and appears only on the wire; a unit test
of the host's asserts no variant carries a `from` event field.

## struct ChannelSpamErrorEmergency
`action` records what the host actually DID, because the lever differs between a
foreground eval and a job that outlived its own.

## struct CriticalEmergency
100% wrong, and the guaranteed-correct response is an MCP restart - the
restart-of-last-resort, which is INTENDED host teardown and the legitimate
counterpart to the shadowed body `exit`. The response is deferred; for now Critical
only logs.
