# link.rs

The link lifecycle over the codec (codec.rs) and the verifier (verify.rs). One
link is two known-public-key mTLS connections to one peer, message and control
on one and files on the other, so file bulk never head-of-line-blocks control:
TCP is one ordered stream, so the separation is two connections rather than a
multiplex inside one. A link opens on demand, after each agent's Channel is
verified; the host runs no listener of its own.

The registries are the system's (`system.rs`): a link's own tasks never touch
them, and tell the system what happened (`LinkEvent`) instead, so every change
to them happens on the system's loop, one at a time.

## struct RemoteLinkEntry
One open link and its peer's address, the system's registry's value: the
listing reads the address, and a close cancels the handle.

### field generation
The connection's own number, from the system's one counter: a dialed link's
drawn as it registers, a listener's its sequence number. Its drivers' end names
it, since a redial under the same alias to the same running peer carries the
same alias and the same nom.

## enum EndedLink
## fn ended_link
A connection's end against what its alias holds now. The alias's own
connection leaves the registry and reports Disconnected; one a close took
first reports Disconnected, the close's own end; and one a newer connection
replaced reports nothing and leaves the newer one alone, since its
Disconnected would read as the newer link's.

## struct BoundListener
A listener bound and waiting for its peer: its address, a sequence number, its
accept's cancel token, and the receiving half of a `released` signal. The
system registers it the moment the bind succeeds, at the address bound, so
`interface remote channel list` can surface a listener that has bound but not
yet paired, and `interface remote channel close` can cancel one.

## enum Up
The relays and the reports, in the order the links make them, for the
system's controller.

## enum LinkEvent
What a link's own task tells the system, which holds the registries: a
connector's dial and its failure, a listener's pairing and its failure, a
closed listener's port released, and a link's end.

### variant Paired
The system moves the link into its links only while the alias still holds this
listener's sequence number, so a close that removed the listener first wins:
the link comes back on `promoted`, and the task cancels it there and then. A
close followed by a re-open can put a newer listener under the same alias,
which is why the sequence number decides rather than the alias.

## struct LinkContext
What every link's tasks share: the way up, an unbounded queue a synchronous
driver path pushes onto without awaiting, and the Channel's inbox as the
controller last told it. A report and a relay keep their order through the one
queue, so a report and a relay reach the agent in the order they were made.

## struct Dial
A connector's dial, on a task of its own so the system's loop serves on while
it runs under CONNECT_TIMEOUT (20s). Its result goes to the system, which
registers the link and spawns only a teardown watcher (`watch_link`); there is
no Connected for the attempt - the connect's answer is the notice. A connect
error or timeout answers remote::channel::open_failed. A dial whose system is
gone cancels its link.

## struct Listen
A bound listener's accept, on a task of its own: accept_and_pair one peer's two
connections, offer the link to the system (`Paired`), and once it is
registered await both joins, then tell the system the link ended. The system
reports Connected {remote, mcp_nom} as it registers the link and Disconnected
as it drops it. An accept failure after a good bind reaches the system, which
leaves only this task's listener and reports Disconnected {remote, error}
(post-bind, so async).

A close cancels the accept's token. The task's `select!` (biased, the cancel
first) drops the accept future, which drops the TcpListener and frees the port,
and only then sends `released`, which the close awaits up to CLOSE_FRAME_GRACE.
So a close answers with the port free, and an immediate re-open of the same
alias binds rather than meeting "address in use". A listener closed before it
paired reports Disconnected {remote}: no `mcp_nom`, since no link established,
and no `error`, since nothing failed.

A connect attempt reports no Connected or Disconnected; its answer is the
notice. Disconnected is kept for any established link's teardown, in both
roles: once told the link is up - Connected for a listener's peer, or the
connect's answer for a connector - a later drop still reports Disconnected, so
a long-running consumer learns the live link died. The system drops the
registry entry only if it is still this task's link, since a later open may
have replaced it under the same alias.

## fn bind_listener
The bind is the address asked for, never another: a port taken is the agent's
to answer, a listen on the ip alone taking the kernel's choice (User).

## fn accept_and_pair
Every peer's connection is taken to the handshake, where the known-key match
decides it; no source address is filtered, the peer being known by its key.

`remote` (the opened alias) is always present on a Connected or Disconnected
packet, so a consumer keys on it and branches on field presence: mcp_nom
present iff the link established, error present iff it failed - absent when
not, never null.

## fn delivery_path
A delivery's file lands at `<inbox>/<peer_nom>/<id>/<dest>`, and all three
parts come from the peer: its nom from its Hello, the id and the dest from
its frames. `Path::join` takes a `..` as written and an absolute part in place
of everything before it, so each part is held before the join: the nom and
the id to base62 (`valid_peer_id`) and the dest to the protocol's `safe_dest`,
the guard a sender applies too. A part that
fails answers no path. Unchecked, a peer naming an absolute id could write any
file this host's user can.
The handshake refuses a nom that isn't base62, and each Deliver and chunk
frame an id that isn't, the Deliver a manifest naming an unsafe dest too,
which would otherwise wait forever for a chunk the receiver drops. A 0.0.0-9
peer mints base62 noms and ids, so the wire is unchanged for every honest
one.

## struct RemoteLinkHandle
The join handles live in the link's own task rather than here, so a cancel
(`interface remote channel close`) lets that task observe the end and tell the
system.

### fn enqueue_send
Registers the send in the SendTracker (its dests and the message) before any
push, so no write outcome can arrive for an unknown id; the pushes are
synchronous mpsc sends. A mid-push failure means the driver is gone (the link
went down at enqueue): the registration is forgotten and the error returned at
once, with no id and no async notice, since nothing left the host. push_frames
is the free helper doing the pushes; splitting it out keeps the
register/forget bracket readable. It checks the Deliver first, since its model
and file names can refuse a whole send before a byte of any payload is copied,
then builds every chunk and checks each as its peer will read it (codec.rs's
`check_outbound`), so a frame the peer would refuse, one inflating past the
cap or one past the decode budget, refuses the whole send with nothing queued,
rather than reaching the peer, which would close the link, after the write had
reported it sent. A frame is checked wrapped in `InitiatorToAcceptor`, whose
`Msg` and `File` frames encode to the same bytes as `AcceptorToInitiator`'s, so
one check serves either end.

## struct SendTracker / struct PendingSend
The aggregate write-outcome half. A send is Sent only when the Deliver and every
file dest's last chunk have written OK across both connections (the whole
transmission), Unsent on the first write error or a link teardown mid-send. This
is not a peer round-trip: the transport write is the ack (TCP and TLS perform
it), so there is no DeliverAck and no receipt timeout. Per-id map: each driver
reports its writes (dest_written on a last-chunk write, message_written on the
Deliver write), completeness is checked under the mutex, and whichever write
finishes the set fires Sent exactly once (the map remove). failed() is
idempotent per id and cancels the shared token so the other connection winds
down; flush_unsent drains the map on each driver's exit, idempotent, so a
teardown reports every still-pending send as Unsent without double-firing. Files
ride the file connection and the Deliver the message connection, so aggregating
"the whole send wrote" needs this shared per-link tracker rather than a single
write site. A driver flushes before its join completes, so a teardown's Unsent
reports queue ahead of the Disconnected the system reports at its end.

## the receive side (RecvContext, DeliverSlot, try_finalize, relay_to_channel)
A delivered message becomes a normal Channel packet (a relay, never an
"injection"). The slot-map per id coordinates the two connections: the Deliver
sets the model, the event, and the expected dest set, each file's last chunk
marks a dest done, and try_finalize fires once (the map remove), from whichever
driver completed the condition, when the Deliver is in and every dest has
landed, relaying it up for the Channel's verification-gated emit, which the
system's controller makes. The receiver sends nothing back - its Sent already
fired from the sender's write, and a refused relay (an unverified Channel among
the causes) is logged on the receiver alone, never reported to the sender as
Unsent. The slot-map is what makes the files exist on disk before the agent
observes the emission. land_file_chunk appends under
<inbox>/<peer_nom>/<id>/<dest> - the same inbox the Channel's own attachments
use, so a relayed packet's `attached` ref resolves for the local agent. The
inbox is read from the context as each chunk lands, since every open makes a
new one: a closed Channel has none and lands nothing, and a delivery that spans
a close and an open loses its earlier chunks with the inbox the close removed.

relay_to_channel refuses a model under `mcp/` or `remote/` first,
`reserved_model`, as the sending grimm commands do: the reservation is what
lets an agent take a model's path as proof of the host's origin, and a peer is
not the local host. Its files have landed by then and stay, as for any refused
relay.

rewrite_spill_pointer: a sender spills an oversized event to the reserved
EVENT_SPILL_DEST file with a dest-relative pointer; relay_to_channel rewrites
that path to <peer_nom>/<id>/<dest> before relaying, so the agent resolves it
as <inbox>/<spilled_event_path> uniformly with a local spill. A no-op for a
normal event or a path already carrying a `/`.

## the McpNom handshake
A Hello variant in each language (codec.rs), exchanged over the same Framed
read/write before the steady loop. The initiator sends first (its nom and the
stream role), the acceptor replies its nom. pair_check requires both connections
of a link to report the same peer nom - a mismatch means the two TCP connections
reached different hosts, which must never become one link. The nom crosses as
its base62 String.

## the driver loops
Two concrete driver fns per side (message and file), `tokio::select!` with
`biased;` and the cancel first, so a close beats an in-flight read
deterministically, the channel hub's close-race discipline. The In/Out enums and
the client-vs-server TlsStream types differ per side, so abstracting them behind
a trait costs more than the duplicated lines buy. close_initiator and
close_acceptor send Close and briefly await the reciprocal (CLOSE_FRAME_GRACE,
2s) so both ends see a clean shutdown rather than a bare drop.

## known-public-key mTLS config
Server authentication alone (`with_no_client_auth` on both sides) would bring a
mutual link up working and unauthenticated, so neither side uses it. Instead:
client = builder_with_provider(ring) .dangerous()
.with_custom_certificate_verifier(PublicKeyVerifier) .with_client_auth_cert(our
public key, our private key) - present ours and match the server's; server =
builder_with_provider(ring) .with_client_cert_verifier(PublicKeyVerifier)
.with_single_cert(ours) - present ours and require and match the client's. Ours
is the host's `[identity]`, the pair its Channel presents too, which the
system's parameters carry; it rides with the peer's in `LinkKeys`. The ring
provider is set by hand so the link never depends on a process default and no
second backend creeps in. PublicKeyVerifier implements both verifier traits
(verify.rs), so one value serves both sides. Single-leaf chain (load_chain =
vec![the public key]): the match uses the leaf DER and ignores any chain.
PLACEHOLDER_SERVER_NAME is a fixed `.invalid` placeholder - rustls needs a
syntactically valid ServerName even though the match ignores it, there being no
real DNS identity under known-public-key trust.

## runtime proof
The TLS lifecycle is exercised at runtime: the system tests' `remote_channel.rs`,
over the harness's `remote.rs`, link two real hosts over mTLS through a
simulated rmcp consumer and run an A->B->A echo - connect, accept, handshake, a
real send and relay, and the Connected, Sent, and Disconnected notice shapes.
Each host's one identity serves its Channel and its link alike. The unit tests
(the codec round-trip, the verifier match) stand alongside it.
