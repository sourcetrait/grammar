# render.rs

A packet's id and its one NUON line, shared by the system's sends and a face's
greeting.

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

## fn escape_line
`to nuon` renders compactly but does not escape a newline inside a string
value, and the client batches frames arriving close together into one event
joined by newlines. So a literal newline in a packet is indistinguishable from
a batch boundary and a reader would see more records than were sent.

Safe because the only raw newlines a compact render can carry are inside
double-quoted strings, where `\n` and `\r` are the escapes nushell reads back -
the line still parses to the original value.

## fn event_overflows / fn event_spill_pointer
The second, lower cap. `MAX_FRAME_BYTES` (1 MiB) is what one wss frame carries,
but the client truncates a task-notification at about 500 characters of the
whole rendered packet line - a claude-code constant the server never observes,
measured at 500. An event between the two transits and fires Sent (the
transport write), yet renders back to the agent truncated, silently and
unrecoverably. So a send whose rendered event would overflow the notification
cap spills: the full event NUON goes to a file and a compact
event_spill_pointer ({spilled_event_path, event_bytes}) rides the wire in its
place, so the agent reads <inbox>/<spilled_event_path> for the real event,
`event_bytes` its original size. event_overflows tests the rendered event's
character count against the cap minus a wrapper reserve (the id, from, model,
attached, and NUON punctuation around it), not the exact line, so one threshold
serves every send path without re-deriving the wrapper. Distinct from the frame
cap: an event over 1 MiB cannot ride a Deliver frame inline, so the remote spill
transfers it through the file connection under `EVENT_SPILL_DEST`, inside the
delivery's own inbox directory, which is why the spill is a send-path act, not a
render choice.

## fn nu_value
A request's NuVal back as nushell's value for the render. It refuses only what
no nushell value could have given, a date or a range outside its type.
