# system.rs

The peer links as a subsys system, and the handle their controller asks
through.

## struct PeerSystem
The registries are its state, changed on its loop alone: each open link by
alias, each bound listener by alias, and each alias being dialed. A link's own
tasks tell it what happened (`LinkEvent`), and their reports and relays queue on
one lane up (`Up`), so the loop takes requests, events, and the lane in turn,
biased in that order.

### field dialing
An alias being dialed is busy, so a second listen or connect on it is refused
`remote::channel::already_open` rather than racing the first.

### field config_file
The agent's config file, named when a link finds no `[identity]`, so the
refusal says what to add.

## struct PeerSysParams
The host's nom, which each handshake names; its key pair, expanded by the
config; the remotes file, read at each command; and the stand-ins a
certificate's unset variables expand to.

## fn PeerSystem::answer
A connect and a listener's close answer later, from a link's task, once the
dial ends or the port is free; every other command answers on the loop.

## fn PeerSystem::rows
The file's entries merged with the live aliases, sorted by alias: a link is
open, a bound listener listening, the rest closed.

## fn PeerSystem::set
Each field is checked as the file will be, and a new entry must name its
certificate, so a set never writes a file the next read refuses.

## fn PeerSystem::to_open
An alias already linked, listening, or dialing is refused; an alias the file
lacks is refused with the command that adds it. The Channel's verification is
the controller's to check first.

## fn PeerSystem::listen
The bind runs on the loop, the listen's synchronous half, so its answer carries
the address bound; the accept runs behind on a task of its own.

## fn PeerSystem::send
The send's reservation is held here as on the sender's side. The id is minted
over the model and the event as rendered, then an event the receiver's cap
would truncate spills into the files under `EVENT_SPILL_DEST`, its pointer
riding the wire in its place, and the frames queue on the link to the peer the
send names.

## fn spill_remote_event
The sender's half of the event spill: an event over 1 MiB cannot ride the
inline Deliver frame at all, so only the file transfer reaches the agent. The
pointer's path is dest-relative; the receiver rewrites it under the delivery's
inbox directory (link.rs's `rewrite_spill_pointer`), so the agent resolves it
as `<inbox>/<spilled_event_path>`, exactly as for a local channel send's spill.

## fn PeerSystem::on_event
Each registry change a link's task asked for, made in the order asked, and
reported as it is made: a listener's pairing reports Connected once its link is
in the registry, and a link's end reports Disconnected once it is out. A dialed
link takes its generation from the counter the listeners' sequence numbers come
from, so no two connections share one, and an end acts on the connection it
names alone (link.rs's `ended_link`).

## fn on_stop
A halt cancels every link and every listener, so none outlives its system.

## struct PeerHandle
### field runtime
The runtime the system was started on, captured at its start. An engine's eval
runs on a thread of its own, outside the runtime, and asks from there by
`request_blocking`; a task inside the runtime asks by `request`.
