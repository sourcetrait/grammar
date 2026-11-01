# width.rs

## fn wide
The wire carries counts and offsets at a fixed 64 bits, whatever the
platform's usize. Widening never alters a value: the assertion beside it fails
the build on a platform whose usize is wider than a u64, so the cast is proven
lossless wherever it compiles. Narrowing goes the other way at each conversion
that needs it, through `usize::try_from`, a value past the platform's usize
refused there rather than truncated.
