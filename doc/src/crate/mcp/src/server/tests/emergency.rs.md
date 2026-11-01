# emergency.rs

## fn no_event_field_collides_with_the_packet_envelope
The packet's envelope carries `from`, the sender, which for anything here is the
host, so an event field of the same name would put two meanings under one word
in a single record. The spam pair names the offender `origin` for exactly that
reason. It is held here because the collision is invisible in the log, where
there is no envelope, and shows only on the wire.

## fn every_model_lives_under_the_mcp_reservation_and_is_distinct
The reservation is worth something only if it holds for every host-originated
model: one variant escaping it turns a mechanical provenance check back into a
judgement call. Distinctness matters because the model is what the agent routes
on.
