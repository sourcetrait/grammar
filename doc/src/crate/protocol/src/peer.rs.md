# peer.rs

The remote Channel's portable data: its models, its rows, and its events, in
the shapes the agent already reads. The links are the MCP's own, apart from the
remote engine: a remote Channel carries events, messages, and files between
hosts, never an engine request, so nothing here names one.

## const MODEL_CONNECTED / MODEL_DISCONNECTED / MODEL_SENT / MODEL_UNSENT
The host's lifecycle and delivery models, under the `remote/` reservation, which
no definition or peer may claim.

## const REMOTE_COMMANDS
The catalog's seven `interface remote channel` kinds, which the MCP serves
whatever the tool's host, the links being its own.

## fn valid_peer_id
A nom or a delivery id is base62, so one is ASCII letters and digits alone, at
most 64 bytes: never a `/`, a `.`, or an absolute path. The mcp crate's link
holds a peer's nom to it at the handshake and a delivery's id at each frame,
since both are joined into the inbox path a delivery's files land under, and
the delivery id is a peer's own text.

## fn safe_dest
A send's dest is a relative path of plain components, never an absolute path,
a `.`, or a `..`, since the receiving link joins it beneath its inbox. A send
is held to it where its files are read, the MCP's or a worker's, and the link
holds every received dest to it again.

## struct RemoteChannelRow
One row an alias, the file's entry beside its live state, so the agent reads
what it configured and what is running in one place: `endpoint` is the bound or
dialed address while live, and `remote_mcp_nom` the peer's nom once paired,
which is what Sent, Unsent, and a relayed packet's `from` carry. Its `state`
serializes as the word the row has always carried, `closed`, `listening`, or
`open`.

## enum RemoteResult
A remote Channel command's result as its tool renders it: `list`'s rows, `get`'s
and `set`'s row, `listen`'s address, and nothing for the other three, each the
kind its catalog entry declares. `answers` ties a result to its command, so a
frontend takes no result for a command that doesn't answer it.

## enum RemoteChannelEvent
Each packet's event as data. `Disconnected` carries `mcp_nom` when a link was
established and `error` when it failed, neither for a listener closed before it
paired, so a consumer keys a link's lifecycle on `remote` and reads the rest by
presence. `Sent` reports the write to the link, never that an agent processed the
message.
### fn event
The record the packet has always carried, a key left out where it has no
value, never null, as the mcp crate's link builds each one by hand.

## struct RemotePacket
A peer's message as a link relays it: the delivery id, the sending peer's nom,
the model, and the payload, data alone. The MCP names the packet
`remote/channel/<nom>`.
### fn check
A relay keeps base62 ids and a model outside both reservations, so a peer can
pose neither as this host's nor as a remote link's report.
