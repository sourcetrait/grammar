# hello.rs

A session's opening exchange, the one place both ends decide whether a session
may carry anything. The worker's check and the MCP's are written beside each
other over the same types, so a fixture run through both ends reaches one
outcome: a hello the worker accepts is a reply the MCP accepts, and a mismatch
one side refuses the other refuses for the same reason.

Nothing in a hello is authority. The SSH session the MCP authenticates, its
pinned host key and its key file, is the boundary, and the worker runs as the
account the session logged in as; the hello's instance and the worker's
identity are metadata a refusal names, so a forged hello reaches no further
than a wrong one.

## struct ProtocolVersion
## const PROTOCOL_VERSION
v1.0. The major must match exactly, and so must the schema ID, so a minor may
differ only beneath one layout: a minor that changed a layout would change the
schema and be refused as a schema mismatch.

## const HELLO_DEADLINE
## const REPLY_DEADLINE
## const AUTH_DEADLINE
The session's finite phases: 5 s for the MCP's hello once a worker's start is
done, 60 s for the worker's reply, a start among it, since a worker reads the
hello only once its lock, scratch, and engine stand, and 30 s for the SSH
session's connection and authentication. The types own no timer: each end
applies these.

## const HELLO_BUDGET
A hello decodes within 64 elements and its frame's 64 KiB of heap, far tighter
than a frame's budget, since it arrives before anything is negotiated.

## struct Instance
An id and a namespace, each held to `valid_name`. A name outside it is
refused, never sanitized, so two names can never be folded into one namespace.

## fn valid_name
An ASCII letter or digit and up to 63 more of those, `.`, `_`, or `-`, which
leaves `.` and `..` out and makes every name one plain path component. The one
rule for an id, a namespace, and a remote engine's alias, the worker's
instance names and the MCP's config holding to it alike.

## struct Limits
The bounds a hello negotiates: the frame ceiling, the heap a frame's payload
decodes within, and the requests in flight at once, the calls apart from the
rest, so a session full of evals still answers a kill or a process list.
`least` takes each bound's minimum, `past` names the first bound above
another's, and `zero` the first at zero.

## const LIMITS_DEFAULT
## const LIMITS_HARD
The defaults and the hard ceilings. A worker configured past the hard ones
still grants none past them.

## struct ClientHello
The MCP's first frame: what it speaks, the namespace it means to reach, and the
bounds it asks for. No role, lease, or feature: a session is one stream with
one purpose, and a build that speaks the version and the schema speaks all of
it.

## struct WorkerHello
## enum HelloReply
The worker's acceptance names its namespace, its UID, its pid, and its nom, the
last drawn at each start, so a log or a refusal names the very process; or a
transport error, after which the worker ends. `Accepted` boxes its hello, some
150 bytes beside a refusal's 56.

## fn agree
The version's major, the schema, and the codec, both ends' first check.

## struct WorkerProfile
What a worker answers a hello from. It holds nothing past what the reply
names.

### fn check
The worker's side: the version, schema, and codec; the expected namespace
valid and the worker's own; no requested bound at zero. It returns each
effective bound, the least of the request, the worker's, and the hard
ceilings.

### fn reply
The reply a checked hello takes, a refusal carrying the check's reason.

## fn check_reply
The MCP's side of the same checks, and what only the MCP can check: the
namespace the one it asked for, the nom base62, and no bound raised past the
request or the hard ceilings or set to zero. A refusal passes through as the
worker's own error, so a held namespace reaches the agent as the worker said
it.
