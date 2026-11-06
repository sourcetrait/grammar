# render.rs

A packet's id, its file, and its one NUON line, shared by the system's sends
and a face's greeting.

## struct MsgId
Distinct from `Nonce` in name rather than in shape: nonces name evals
throughout grammar, so reusing the word on the wire would confuse two different
things.

## fn mint_msg_id
The attached content is hashed, never its path - the path is derived from the
id, so hashing it would be circular. The generator mixes in a counter and a
timestamp, so two byte-identical packets still get distinct ids.

## fn render_nuon
Rendering from a Value is deterministic, which is what lets the id hash cover
the rendered form.

## struct RenderedPacket / fn render_packet
The second, lower cap. `MAX_FRAME_BYTES` (1 MiB) is what one wss frame carries,
but the client truncates a task-notification at about 500 characters of the
whole rendered packet line - a claude-code constant the server never observes,
measured at 500. A line between the two transits and fires Sent (the transport
write), yet renders back to the agent truncated, silently and unrecoverably.

So the packet is rendered whole for its file, `<id>.nuon`, and the line is held
to the cap as it stands, every field it carries counted: the event, a peer's
model, and the `attached` table, whose names a peer chooses. A line past the cap
spills whole: the wire carries the event as `{size}`, the file's size, and no
`attached`, since the agent reads the file for all of it. A real event of the
same shape needs no telling apart, since the file holds the packet either way.
The spilled line is short by construction, its model held to `MODEL_MAX_CHARS`
where a sender or a relay takes it.

## fn packet_value
`attached` is a table of each file's path in `<id>/` and its size, never a
directory: the agent has the Channel's directory from its open and the id from
the packet. It is left out where there are no files, as `<id>/` is.

## fn escape_line
`to nuon` renders compactly but does not escape a newline inside a string
value, and the client batches frames arriving close together into one event
joined by newlines. So a literal newline in a packet is indistinguishable from
a batch boundary and a reader would see more records than were sent.

Safe because the only raw newlines a compact render can carry are inside
double-quoted strings, where `\n` and `\r` are the escapes nushell reads back -
the line still parses to the original value. The file keeps the render as it
is, a NUON file taking raw newlines in its strings.

## fn event_overflows
A remote sender's own choice, made without the receiver's line: an event past
the cap less a wrapper reserve rides the file connection under
`EVENT_SPILL_DEST`, which also carries an event over 1 MiB that no Deliver frame
could. The receiving Channel renders the packet from that file and holds its own
line to the cap, so the sender's guess never decides what the agent sees.

## fn event_spill_pointer
The size of the packet's file, the one field a spilled event keeps.

## const ATTACHED_FILE / fn event_file
One name for each in every path that emits, so the agent finds any packet's file
and a local attachment without asking which path sent it.

## fn nu_value
A request's NuVal back as nushell's value for the render. It refuses only what
no nushell value could have given, a date or a range outside its type.
