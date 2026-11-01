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
system mints the id, spills an oversized event, and queues the frames.

## struct InboxRequest
The Channel's inbox, which each open makes and a deliberate close removes. The
controller tells the system at both, so a link lands its files where the
Channel's attachments sit, and the answer comes once the links hold it, so a
close removes the directory only after no link reads it there.

## struct Relay
A peer's message for the Channel, and whether its files landed beside it.

### fn attached
The inbox-relative directory the relay's files landed in, `<origin>/<delivery>`,
which its packet names.

## enum FromPeerSys
### variant Relay / variant Report
Sent up unasked, in the order the links made them: a relay is a peer's message
for the Channel's gate, a report a link's lifecycle or a send's delivery.
