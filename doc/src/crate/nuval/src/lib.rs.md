# lib.rs

nuval is the standalone NUON library: the value, its NUON text, and serde
types to and from both, for any crate that wants NUON, with nushell's own
values beside them behind a feature (User).

## feature nushell
nushell's own values and errors, converted to and from nuval's, take
nu-protocol, miette, and chrono. The feature is on by default and gates every
nushell dependency (User). The data and its text take none of them: the workspace takes nuval
with default features off, so the protocol and the client carry values and
their text without evaluating anything, and the engine and the MCP crate name
`nushell`. Each conversion sits beside its data behind
`#[cfg(feature = "nushell")]`; a kind's code and message, an error's `short`,
every type's codec derives, and the NUON text need no feature.

## mod kind
One file a nushell error type's family of kinds, the error model's tables.

## mod text
NUON text without nushell: the reader and the writer port nushell's own, and
the serde serializer and deserializer carry any type to and from a NuVal and
so to and from the text.

## use crate::error::*
## use snafu::OptionExt
The conversions' error context, behind `nushell`: `context` on an `Option`
for a range's second value overflowing and a row index past the platform's
usize. snafu itself is in every build, since the NUON error derives with it.

## mod nu
nushell's items the conversions name, under one path, so a reader tells them
from nuval's own, behind `nushell`. nushell's I/O error kind is `IoKind`
here, apart from nuval's `IoErrorKind`.

## use sourcetrait_common::cereal
cereal comes through the workspace's `sourcetrait_common`, its facade feature
set, rather than `sourcetrait_cereal` alone (User). `cereal::derived` emits
absolute `::serde`, `::bitcode`, and `::rkyv` paths, so those three are the
crate's own dependencies, at the instances cereal uses; miette and chrono are
taken at nu-protocol's.

## pub use
The value, its error and result, the data each kind of value holds, every
family of error kinds, and `wide`; the NUON text's style, error, and serde
conversions; with `nushell`, the two conversion traits and the conversion's
error.
