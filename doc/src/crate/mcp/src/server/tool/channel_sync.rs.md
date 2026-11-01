# channel_sync.rs

## fn channel_sync
A sync channel's whole delivery, User's design for a harness with no channel mechanism:
the agent calls it when it chooses and takes every packet queued since its last call,
oldest first, with `dropped`, the packets the cap pushed out meanwhile. The first call
also claims the channel (`take_sync`, crate/channel's `state.rs`).

An async channel refuses it as `channel::wrong_mode`, its packets going to its Monitor,
and a closed one as `channel::not_open`, as a send would be.

## fn packet_record
Each queued line is the rendered packet the wire would carry, read back by `from_nuon`
into its record, so a packet reads the same whichever mode carried it, a spilled event's
pointer included. `attached`, absent from a line naming no file, is null here, so the
packets form one table.
