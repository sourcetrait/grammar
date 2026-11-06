# render.rs

A packet's id, its line, and the files its spill takes, shared by the system's
sends and a face's greeting.

## const DATA / const ATTACHED
The envelope's two variable fields. Every packet is `mcp/channel/Envelope`, one
of two closed records: `{id, from, model, data}`, or the same with `attached`
where the packet attaches anything. A type definition holds no optional field,
so a packet with nothing attached is the first variant rather than the second
with its field left out or null.

## struct MsgId
Distinct from `Nonce` in name rather than in shape: nonces name evals
throughout grammar, so reusing the word on the wire would confuse two different
things.

## fn mint_msg_id
The attached content is hashed, never its path - the path is derived from the
id, so hashing it would be circular. A relay's attachment is hashed as its
table, the names and sizes of the files already on disk. The generator mixes in
a counter and a timestamp, so two byte-identical packets still get distinct ids.

## fn render_nuon
Rendering from a Value is deterministic, which is what lets the id hash cover
the rendered form and a sender render the line its receiver will.

## enum Attached
What a packet attaches, as its line first carries it. A relay's files are
inline as their table and spill to `<id>.attached.nuon`'s size; a local send's
value is any NUON, never a table of files, so its line always carries the
file's size. `bytes` is the size of the file the caller writes, which the spill
names.

## struct RenderedPacket / fn render_packet
The second, lower cap. `MAX_FRAME_BYTES` (1 MiB) is what one wss frame carries,
but the client truncates a task-notification at about 500 characters of the
whole rendered packet line - a claude-code constant the server never observes,
measured at 500. A line between the two transits and fires Sent (the transport
write), yet renders back to the agent truncated, silently and unrecoverably.

So the line is held to the cap as it stands, every field counted, and spilled in
the order that keeps the most on it: `attached` first, to its file's size, then
`data`, the most important, to `<id>.data.nuon`'s size, the text handed back
for the caller to write. Only what spills is written; the inbox is no record of
every packet. The envelope itself never spills: past both spills only `id`,
`from`, and `model` remain beside two filesizes, `model` held to
`MODEL_MAX_CHARS` and an engine's alias, `from`'s one long part, to 128
characters, so the line is about 360 characters at its longest. An envelope
past the cap even so is refused, never truncated.

A spill is a bare filesize. No send takes a filesize as its data, so a real
record or table can't be mistaken for one, as a `{size}` record could.

## fn line_fits / const ID_LINE_MAX
A sender that must keep a field on the line, a chat its summary, renders the
packet its receiver will before the receiver mints the id: `attached` in its
spilled form, since the receiver spills it before `data`, and the id empty and
counted at its longest, eleven base62 digits quoted. Data a line passes here
stays on the receiver's line whatever id it mints.

## fn envelope
The record both variants share, `attached` placed only in the second.

## fn attached_table
`file` is each file's path in `<id>/`, never a directory: the agent has the
Channel's directory from its open and the id from the packet. The same table is
the line's and `<id>.attached.nuon`'s, so a spilled one reads back whole.

## fn escape_line
`to nuon` renders compactly but does not escape a newline inside a string
value, and the client batches frames arriving close together into one event
joined by newlines. So a literal newline in a packet is indistinguishable from
a batch boundary and a reader would see more records than were sent.

Safe because the only raw newlines a compact render can carry are inside
double-quoted strings, where `\n` and `\r` are the escapes nushell reads back -
the line still parses to the original value. A spill's file keeps the render as
it is, a NUON file taking raw newlines in its strings.

## fn data_file / fn attached_file
The two names a packet takes beside it in the Channel's directory, so no name in
a relay's `<id>/` is the host's and the agent finds a spill or an attachment
without asking which path sent it.

## fn nu_value
A request's NuVal back as nushell's value for the render. It refuses only what
no nushell value could have given, a date or a range outside its type.
