# limits.rs

The session's frame ceilings and decode budget. The other bounds, the
deadlines and the requests in flight, belong to the hello and the session
that enforce them.

## const HELLO_FRAME_MAX
A session's first frame each way is its hello, before anything is negotiated,
so both directions start under this ceiling.

## const FRAME_DEFAULT
## const FRAME_HARD
The default until the hellos negotiate, and the ceiling no negotiation
passes: the MCP asks for less, never more.

## type DecodeBudget
bitcode's `Budget` from sourcetrait's fork, so the bounds a payload decodes
within are the ones its decoders charge, each checked before what it bounds
is allocated. The heap is what the decoded value keeps; the decoder's own
column buffers, freed when the decode ends, are outside it. An element is a
collection's item, a byte buffer's or a string's bytes charged as heap
instead.

## const DECODE_BUDGET_DEFAULT
100,000 decoded elements and 32 MiB of decoded heap. Its depth is 128 where a
value's own limit is 64: the decode counts every collection that nests with
values in it, a frame's own among them, so a value's 64 levels fit inside a
frame's containers. Those 64 are `check_value`'s. 128 levels of decoding take
well under a two-MiB worker stack. A worker holds each frame it sends to the
session's budget before it queues it, so a result past it is refused alone
rather than breaking the MCP's decode.

## const DECODE_HEAP_HARD
The heap a negotiated budget may reach.
