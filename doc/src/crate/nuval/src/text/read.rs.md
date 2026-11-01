# read.rs

nushell reads NUON with its whole parser. This is that parser's NUON path
ported, so nuval reads what `from nuon` reads and refuses what it refuses,
without nushell's crates. `src/tests/text.rs` holds it to nushell on
nushell's own text and on hand-written text at the grammar's edges, refusals
included.

## impl NuVal
### fn from_nuon
One value: empty text is nothing, and a second value is an error. A bare word
at the top reads as a command in nushell, so it is refused there and only
there; inside a structure it is a string.

## struct Lexer
nushell's `lex_item`: a token runs to whitespace or the context's separators,
held together by brackets, quotes, and raw strings. A `#` opens a comment at
a token's start, or after whitespace inside brackets.

## fn value
nushell's `parse_value` for any shape. The leading byte routes a cell path, a
record, a list, or a raw string; then binary, a range, a filesize, a
duration, a date, an int, a number, and a string are tried in that order. A
miss lets the next shape try and any other failure ends the read, as
nushell's `Expected` errors and the rest divide.

## fn record
Keys are lexed with `:` as a token of its own and values without it, so a
value can hold a colon; a bare value holding one is still refused, nushell's
guard against a missing space.

## fn list
A list whose first item is a list and whose second token is `;` is a table.

## fn string
A double-quoted string resolves nushell's escapes byte by byte, so `\x`
escapes can spell UTF-8. A bare string holding `(` is an interpolation, which
NUON refuses.

## fn int
A prefixed int is read as a u64 and cast, nushell's way to write
`0xffffffffffffffef`.

## fn range
The bounds are numbers. Ints alone make an int range and any float a float
range, with nushell's step rules: a float range under 1 wide steps by the
power of ten below its width.

## fn unit_value
nushell's `parse_unit_value`: a whole unit's fraction is carried into the
smaller unit before both reach the base, and a fraction of the base unit is
dropped. A token that starts like a number and ends in a unit but isn't one
is a failure, not a string, as in nushell: `3rdb` doesn't read.

## fn datetime
RFC 3339 as chrono's parser reads it, then a date alone at midnight UTC, then
a date and time with `+00:00` appended: nushell's three tries.

## fn cell_path
`$.` and its members, `!` and `?` in either order after each; a quoted member
is a name even when it is digits.
