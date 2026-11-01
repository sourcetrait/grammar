# msg.rs

The engine's own messages. The requests, commands, purview requests, and
answers are the protocol crate's, portable data every endpoint shares, and ride
whole inside these, so a session carries them to a worker's engine as a local
controller sends them to its own. What stays here needs the engine: the asks an
eval makes of the process hosting it, which never cross as they are, and each
request naming its answer type for subsys against `EngineSystem`.

## enum ToEngine
## enum FromEngine
A subsys system takes one type each way, so the wire's pair rides one variant,
the purview requests another, and the host's a third: down, a request or a host
answer; up, an answer or a host ask.

## impl subsys::Request for PurviewRequest
A purview request whole answers the purview answer whole, so a controller that
routes requests by host, the MCP's to its own engine or a worker's, sends one
request and reads one answer whichever engine serves it.

## macro purview_request
Each purview request names its answer, so a controller gets its own type back.

## enum HostAsk
What an eval asks of the process hosting it. The Channel, the remote links,
the configuration, and the pins are the host's, and the engine reaches them
only by asking its controller. A body's send carries its data, the eval's stop
held back on its own side. The stop's report is an ask of its own, since the
action it reports is known only once the sender has stopped. A worker's
controller answers a pin itself and carries the rest to its session's MCP as
the session's asks, a remote send prepared first (`send.rs`).

## macro wire_request
`ExecuteRequest`, `RerunRequest`, and `CallRequest` answer with a `ValueAnswer`,
`ControlRequest` with an `EngineResult<ControlAnswer>`, each wrapped as
`ToEngine::Wire`. The trait is subsys's and the request types are the
protocol's, so the impls stand here because `EngineSystem` and `ToEngine` are
this crate's: the orphan rule admits an impl whose trait parameter or target is
local.
