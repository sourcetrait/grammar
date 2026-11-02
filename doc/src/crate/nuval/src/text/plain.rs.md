# plain.rs

NuVal's derived serde writes its variant, `{Record: [...]}`, the form its own
round trip needs. A field a reader sees takes this module instead, through
`#[serde(default, with = "nuval::plain")]` on an `Option<NuVal>`, so the value
shows as itself: a diagnostic's `detail` record reads as the record it is.

## fn serialize
None is nothing; a value is written as the value it holds.

## fn deserialize
Nothing reads as none, any other value as its NuVal; with `default`, a missing
field is none.

## struct Plain
Each kind as `from_nuval` hands it to serde, so a value written here reads
back through a NuVal unchanged in its form: a record a map, a list a sequence,
a filesize and a duration their ints, a glob its pattern, a date, a range, and
a cell path their raw NUON text. Those kinds serde has no form for come back
as the int or the string they were written as.

## struct ValueVisitor
Each serde kind read as its NuVal. A u64 past an int's range is refused, never
wrapped.
