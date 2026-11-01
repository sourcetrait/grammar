# send.rs

Grimm decls a definition calls, registered by `register_nuapi`, never MCP tools: an
agent's willful, non-blocking send to a linked peer. Both ask the engine's
controller (`HostAsk::RemoteSend`), whose peer links mint the message's id, find
the open link by `mcp_nom`, queue the send, and answer the id at once as
`{event_id}`. The delivery's outcome lands later on the sender's own Channel as
remote/channel/Sent or Unsent, driven by the transport's write (`crate/peer`'s
`link.rs`): Sent covers the whole transmission, files and message.

## struct GrimmRemoteChannelSend / struct GrimmRemoteChannelSendWith
Two decls, not one with an optional attachment, because for a remote send the
transferred files are the attachment, `_send_with`'s whole job, so a bare send has
no files at all, unlike channel send's optional `attached`. `attached` is the files
table, each local `src` beside the `dest` the receiver lands it under.

## fn remote_send
The reservation is refused first, as the send always refused it before reading
anything, then the event and the files table cross as data; the controller reads
each file, rendering the event after (`crate/mcp`'s `server/host.rs`). A model under
`mcp/` or `remote/` is host-origin only, as for a channel send.

## fn shell_error
As channel send's: the command titles a refusal, and the host's reason labels it.
