# val_error.rs

The error model User asked for: a struct splitting out a kind, with a static
message for each built-in nushell error kind, the generic serializable error
for anything nushell raises.

## struct NuValError
Its fields follow nushell's own serializable error, `LabeledError`, which is
how nushell serializes any `ShellError`: `msg`, `code`, `labels`, `help`, `url`,
and `inner`. It adds what a `LabeledError` loses: `kind`, which error it was;
`severity`, a warning apart from an error; `cause`, the diagnostic source
miette renders under the error, an I/O error's context or a generic error's
source; and `source`, the text an error carries itself when its labels index
that text rather than the engine's source.

`msg` is the error's Display, `labels` and `help` its miette parts. A label's
span is an offset into the engine's whole source, or into `source` when the
error carries one; a reader maps it to a position, since only the reader
knows where its own text sits in the engine.

### fn short
nushell's own short report style, a message, then its labels, then its help,
made for a reader on one line: a message's closing period gives way to the
labels' colon, the cause follows a dash, and each inner error follows a
semicolon. Whitespace runs collapse, so a help written over lines stays on
one.

A label its message already ends with is dropped, case and a trailing period
aside: nushell labels many errors with their own words, "File not found" over
"File not found" and "Parse mismatch: expected record." over "expected record",
which one line would read twice. A label adding words, a did-you-mean among
them, stays.

## enum NuValErrorKind
One family per nushell error type, the variant's kind inside it, so a kind
names the type and the variant it was. A shell error wrapping another type
takes that type's family: `Io`, `Network`, `Job`, `Generic`, and `Labeled`. A
chain's first link is the error it wraps, its own kind; a later link is
`Chained`. `Other` is a diagnostic from outside nushell's types, made by
`from_diagnostic`.

### fn code
A kind's code where nushell declares a static one. `Generic`, `Labeled`, and
`Other` carry their code as data alone, since their source sets it. Several
nushell variants share a code, so a code doesn't name a kind; the kind does.

## enum NuValSeverity
miette's severity, an error where nushell gives none.

## fn shell_inner
A shell error's related errors converted by their concrete types, so each
keeps its own kind: miette hands related errors over as bare diagnostics,
which would make every one `Other`. The variants are the ones nushell marks
`#[related]`.

## fn chain_is_first
`ChainedError` keeps whether it is the first link in a private field, and
behaves as its source when it is. Its derived Debug output begins with that
field, so a bounded writer reads the prefix and stops, never rendering the
chain.

## impl From<miette::LabeledSpan> for NuValErrorLabel
A label's end is its offset and its length added at the wire's width, the sum
saturating rather than wrapping: a span can't reach the top of a u64 over real
source text, and a wrapped end would land before its start.

## fn nu_span
A span back in nushell narrows both offsets through `usize::try_from` and
refuses one ending before it starts, as `Span` from the wire: nushell's own
`Span::new` asserts the order, and a peer's data must not reach that assert.

## impl TryFrom<NuValError> for nu::LabeledError
Back in nushell an error is a `LabeledError`, as nushell itself deserializes
any shell error. The kind, severity, cause, and carried source have no place
there and are dropped. It is fallible because each label's span and each inner
error's narrow, so a span the platform can't hold is refused, never cut.
