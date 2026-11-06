# send.rs

Grmr decls a definition calls, registered by `register_nuapi`, never MCP tools: an
agent's willful, non-blocking send to a linked peer. Both ask the engine's
controller (`HostAsk::RemoteSend`), whose peer links mint the message's id, find
the open link by `mcp_nom`, queue the send, and answer the id at once as
`{event_id}`. The delivery's outcome lands later on the sender's own Channel as
remote/channel/Sent or Unsent, driven by the transport's write (`crate/peer`'s
`link.rs`): Sent covers the whole transmission, files and message.

## struct GrmrRemoteChannelSend
One decl: `data` the message, and `attached`, optional, the files table, each local
`src` beside the `dest` the receiver lands it under, left off where the send carries
no files. `data` is the envelope's own word for what a packet carries; the wire's
`RemoteSendData` keeps its field `event`, so its schema stands.

## fn remote_send
The reservation is refused first, as the send always refused it before reading
anything, then the data and the files table cross as data; the controller reads
each file, rendering the event after (`crate/mcp`'s `server/host.rs`). A model under
`mcp/` or `remote/` is host-origin only, as for a channel send, and one past
`MODEL_MAX_CHARS` is refused beside it, as the receiver would refuse its relay.

## fn shell_error
As channel send's: the command titles a refusal, and the host's reason labels it.
