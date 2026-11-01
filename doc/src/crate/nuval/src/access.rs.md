# access.rs

A value is read by its parts as serde_json's `Value` is, so a caller holding
a NuVal, a tool's envelope in a test above all, reads it as it read JSON:
an index, `get`, an accessor by kind, and a comparison with a Rust value.

## static NOTHING
An index past a value's parts finds nothing rather than panicking, so a
chain like `env["result"]["status"]` reads to its first miss and a test
reports the comparison that failed.

## impl NuVal
### fn as_f64
A float, or an int as one: a number read as a float takes either, as
nushell's `number` does.

### fn as_u64
An int at zero or above alone, so a count or a size never reads a negative
as a huge number.

## impl PartialEq
A value equals a Rust string, int, or bool of its own kind alone: an int is
never its text, and a float never an int, as NUON keeps them apart.

## impl fmt::Display for NuVal
The raw NUON, on one line, so a value in an assertion's message reads as
the wire carried it.
