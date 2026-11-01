# val.rs

The value model began as User's prototype in `sourcetrait_nuin`,
`src/val/data.rs`, carried verbatim. `Val` is named `NuVal` so it imports
without colliding with the many crates that name a value `Val` (User);
`ValError` and `ValResult` follow it. The prototype closed with nushell's own
`Value` enum as a comment, a reference for what to extend; the conversions
match `Value` exhaustively now, so they hold that reference and the comment
is gone.

## type NuValResult
A 1:1 of the serializable `nu_protocol::Value`, split in two: every value but
`Value::Error` is `Ok`, and the error is the `Err`, so an error travels beside a
value as a result rather than inside it as one.

## enum NuVal
nushell's `Value` less three variants. `Error` is the result's other half.
`Closure` holds a block and captures that belong to the engine that made them,
and `Custom` a plugin's own type, so neither means anything outside its engine
and neither is carried. `Record` keeps its pairs in order, as nushell's record
does. `Filesize` and `Duration` are nushell's own integers, bytes and
nanoseconds.

## struct DateData
The instant as seconds since the Unix epoch and their nanoseconds, with the
offset in seconds east of UTC. The prototype's fields mirrored chrono's
internals, a day's seconds and the date's packed year and ordinal, which
chrono's public API can't rebuild a date from. `seconds_fraction` can reach
two seconds' worth of nanoseconds, chrono's form of a leap second, and comes
back unchanged.

## struct SpanData
A span as data, the error model's labels' spans. The prototype's first error
model, `ErrorData`, a code, a message, and one span, gave way to the struct in
`val_error.rs`.

## fn int_range
nushell's `IntRange` and `FloatRange` keep their fields crate-private, so a
range comes back through their serde form, the one constructor that takes a
step exactly: `Range::new_float` rebuilds a step as `next - start`, which a
float doesn't always survive (`0.1 + 0.2 - 0.1` isn't `0.2`). The serde form
skips the checks `IntRange::new` makes, so they are made here, the same ones:
a step of zero would iterate forever, and a step running away from its end is
refused as nushell refuses it.

## fn float_range
`FloatRange::new`'s checks, the same way: a finite start, a second value
finite and apart from the start, and an end that is a number; an infinite end
comes back unbounded, as nushell makes it.

## fn try_into_nu
A row index is u64 on the wire and usize in nushell, so going back it narrows
through `usize::try_from` and a value past the platform's usize is refused as
`Index` rather than truncated. Coming in, `wide` widens nushell's usize, which
never alters it.

## const SPAN
A value carried as data has no span, its source gone, so a member converted
back to nushell takes the unknown span unless a caller gives one.

## trait TryIntoNu
Every part of a value converted back takes the one span the caller gives,
usually the call that produced it, since the data kept none of its own.

## impl TryFromNu for NuValResult
A value that is an error, at the top or nested, makes the result that error,
converted by its own type.

## impl TryIntoNu for NuValResult
An error result becomes nushell's error value, its error a `LabeledError`.

## impl TryIntoNu for NuVal
A record naming a column twice is refused rather than collapsed: nushell's
records can't hold two, and taking either would drop data silently.
