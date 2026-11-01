# write.rs

nuon 0.116.1's writer ported onto NuVal, so nuval's text is nushell's byte for
byte without nushell's crates. `src/tests/text.rs` holds it to `to_nuon` in
every style over generated values.

## enum NuonStyle
nushell's `ToStyle`, the four layouts `to nuon` writes. Commas stay, as
nushell writes them by default; `to nuon`'s options for no commas, raw
strings, and lists of records aren't carried, since nothing here writes them.

## fn text
A string standing alone is quoted, as nushell quotes one in command position;
inside a structure `nested` writes it bare where it reads back as itself. A
glob is written as a quoted string, as nushell writes it, so it reads back as
a string.

## fn list
A list of records with the same columns in the same order is a table, as
nushell writes one. The indented styles align its columns, widths measured in
bytes and padded in characters, as nushell's are.

## fn needs_quoting
nushell's quoting regex without the regex: its digit class is Unicode's
decimal digits, hence the table, and its whitespace Unicode's, which
`char::is_whitespace` is.

## fn rfc3339
chrono's RFC 3339 writer without chrono, which the portable build doesn't
take: four digits of year from 0 to 9999 and a sign outside them, the
fraction in the fewest of 0, 3, 6, or 9 digits, a leap second's extra
nanoseconds as second 60, and the offset rounded to the minute.

## fn civil
Howard Hinnant's days-to-civil algorithm, proleptic Gregorian with a year 0,
as chrono counts.
