# msg.rs

The peer links' requests, their answers, and what they send up unasked. Each
remote command answers the wire's `RemoteResult` or its `RuntimeError`, the
same pair the runtime's remote commands answer, so a controller renders either
alike.

## struct ExpandDefault
A variable's stand-in when it is unset, so a certificate's path expands in the
system as the controller's own expansion would, the controller's home model
crossing as data.

## struct RemoteConnectRequest
Answers once both connections are up or the dial has failed; the dial runs on a
task of its own, so the system serves on meanwhile.

## struct RemoteCloseRequest
A listener's close answers once its accept let the port go, so an immediate
re-listen binds.

## struct RemoteSendRequest
A body's send with its event rendered and its files read on the sender's side:
a source the sender can't read is its own error, surfaced at the call. The
system mints the id and queues the frames, the event whole in its Deliver frame.

## struct HostSendRequest
The host's own send, a chat, apart from a body's so the reservation a body's
send is held to stays unconditional: no flag on a body's request can lift it.
Its model must lie under `channel/`, the one prefix the host sends a peer.

## struct InboxRequest
The Channel's directory, which an open makes or takes and a deliberate close
removes. The controller tells the system at both, so a link stages its files in
the Channel's own directory, and the answer comes once the links hold it, so a
close removes the directory only after no link reads it there.

## struct Relay
A peer's message for the Channel, and the files it staged, sorted by name so a
packet's `attached` reads the same however its chunks landed.

## struct RelayFile
A landed file's name and size, read by the link where the file was staged, so
the Channel's loop stats nothing.

### fn staged
The inbox-relative directory the relay's files are staged in,
`.incoming/<origin>/<delivery>`: the peer's nom keeps two peers' deliveries
apart until the Channel mints their packets' ids.

## const INCOMING_DIR
A dotted name, so the staging sorts apart from the packets' files in the flat
directory, and a deliberate close removes it with them.

## enum FromPeerSys
### variant Relay / variant Report
Sent up unasked, in the order the links made them: a relay is a peer's message
for the Channel's gate, a report a link's lifecycle or a send's delivery.
