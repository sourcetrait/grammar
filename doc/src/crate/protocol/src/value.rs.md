# value.rs

## struct ValueLimits
The wire contract's version-one bounds for one decoded payload: collections
nested 64 deep, 100,000 elements, and a 32 MiB heap budget. Bytes count the
variable-sized parts, text and binary, the heap a hostile payload inflates; the
fixed-size scalars ride inside the frame's own bound.

## enum ValueLimitError
Which bound a value passed, with the bound, so a refusal names what to shrink.

## fn check_value
The walk keeps its own stack, so a value nested past any bound is refused
without recursing once a level: a recursive walk would hand a hostile depth to
the thread's stack, the failure the bound exists to stop. Depth counts lists
and records, the collections that nest; a top-level list is depth 1. Elements
are counted over the whole value, items, fields, and members together, and
bytes the same way.

The check serves two places. A worker holds a result to the negotiated bounds
before sending it, so an oversized result is refused as too large rather than
cut. A decoder applies the same bounds while it decodes, before allocating,
since a check after a full decode would already have spent the memory.

## fn check_error
An error nests through its inner errors, so the same walk holds its depth, and
its texts and source count as bytes, its labels and inner errors as elements.
