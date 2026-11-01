# fixture.rs

The v1 fixture driver, the one interface a native client's or daemon's tests
decode the golden vectors through. Each vector's row names its type, so a
consumer needs no knowledge of the test that wrote it, and the driver shares
the DTOs alone, never a runtime.

## const FIXTURE_TYPES / fn canonical
Every type a v1 vector names, each with its decoder: `canonical` decodes
within a budget and encodes the value back, so a vector is canonical when its
bytes come back unchanged. bitcode accepts more than one byte form of some
values, so a decoded value's canonical form, not the bytes it came in, is what
a property is held to. The gate test holds the list to exactly the types the
vectors name.

## fn read_vectors
A vectors file's table, its hex read as bytes, refused for a row missing a
field or hex not two digits a byte.

## fn digest
The SHA-256 of a file's value rendered canonical, so a checkout's line
endings and a file's layout leave it where it was. The manifest pins each
file by it.

## fn check_vector
The round trip, then each proper prefix and a trailing byte, each of which
must be refused: a truncated or padded frame never decodes as a value.
