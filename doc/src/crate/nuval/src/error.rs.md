# error.rs

## enum NuonError
One error for the text and the conversions alike, so a caller reading a serde
type from NUON meets one kind whether the text or the type refused.

### Read
The offset is the token's start in the whole text, a nested value's
included, since the reader keeps absolute spans as nushell's parser does.

### Convert
### fn within
The path is built as the error unwinds, each record field and list index
prepended, so `nested.0.value.key` names the value serde refused. serde's own
messages, a missing field or a wrong type, arrive with an empty path, which
the unwinding fills.

## enum ConvertError
Each variant is a value nushell holds that nuval can't, or data nuval holds
that nushell wouldn't make.

### ErrorValue
A value that is itself an error carries the `ShellError` whole, so the
conversion of a result can take it as the result's error rather than refuse
it. A value nested anywhere in a list or a record counts: nushell raises such
an error when the value is used, so the whole result is that error.

### Index and Span
The data's offsets are u64 on the wire, nushell's usize in the evaluator. A
row index or a span past the platform's usize is refused rather than
truncated, and so is a span ending before it starts, which nushell's own span
asserts against: data from a peer meets the same check as data from the
engine.
