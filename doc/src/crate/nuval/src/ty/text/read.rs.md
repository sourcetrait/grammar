# read.rs

nushell's `parse_shape_name` and its parameter readers ported, so a type reads
as an annotation in nushell reads and is refused where one is. The tests hold
it to nu-parser on type text at the grammar's edges.

## impl NuType
### fn from_text
`list` alone is `list<any>`, and `record`, `table`, and `oneof` alone hold
nothing, as in nushell. A type name runs straight into its `<`: `list <int>` is
two tokens to nushell, and refused.

## struct Reader
### fn fields
A field without a type is `any`, `record<name>`, as nushell reads one; commas
and newlines both separate. A field named twice is kept twice, as nushell
keeps it.

### fn field_name
A key reads as nushell's string parse reads it: double-quoted with escapes,
single-quoted or backticked as it stands, or bare, `true`, `false`, and `null`
bare refused.
