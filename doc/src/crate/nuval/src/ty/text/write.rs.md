# write.rs

## impl NuType
### fn to_block
The layout the repository's `.nutype` files keep: a record or a table with
fields takes a line a field, four spaces a level, and a list or a union takes
lines only where it holds one, so `oneof<string, nothing>` stays inline. The
tests write each of the repository's `.nutype` files back byte for byte.

## fn field_name
A bare key is one nushell's string parse reads back as itself: letters,
digits, `_`, and `-`, and no keyword; any other is double-quoted.
