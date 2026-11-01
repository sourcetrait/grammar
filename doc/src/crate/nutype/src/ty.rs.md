# ty.rs

## enum NuType
The types a value nuval holds can take, and the shapes nushell's annotations
give them; a closure, a block, and an error are left out, since no NUON value
is one.

`path` and `directory` stand as variants of their own though nushell's `Type`
folds both into `string`: a type read from a signature's `SyntaxShape` keeps
what its author declared, and `resolved` gives the type a value of it has
(User: "we have enums for things like `path` `directory` etc even if they
resolve to `string`. the enum simply handles that internally").

An EMPTY field list is an open record or table, nushell's own spelling of one.

`Recursive` tells cereal of the cycle through `NuField`, which its own
detection, a type naming itself, doesn't see.

### impl ser::Serialize / impl de::Deserialize
serde carries a type as its text, so NUON and every text format write it as
nushell spells it, `record<a: int>`; bitcode and rkyv carry its structure for
the wire. cereal's serde derive is off for it.

## struct NuTypeFile
A `.nutype` file: the leading comment is the model's documentation, kept line
by line so a file written back reads as it was.
