# lib.rs

nuval is the standalone NUON library: the value, its NUON text, and serde
types to and from both, for any crate that wants NUON, with nushell's own
values beside them behind a feature (User). A nu type is the same concept's
other class of model, so the types live here too: the type model as cereal
data, so a schema crosses the wire in bitcode or rkyv, its text, read and
written as nushell spells types, so a `.nutype` file is its schema, a value
held to a type, and nushell's own shapes and types converted both ways
(User).

## feature nushell
nushell's own values and errors, converted to and from nuval's, take
nu-protocol, miette, and chrono. The feature is on by default and gates every
nushell dependency (User). The data and its text take none of them: the workspace takes nuval
with default features off, so the protocol and the client carry values and
their text without evaluating anything, and the engine and the MCP crate name
`nushell`. Each conversion sits beside its data behind
`#[cfg(feature = "nushell")]`; a kind's code and message, an error's `short`,
every type's codec derives, the NUON text, and a nu type's model, text, and
check need no feature, its conversion to nushell's shapes and types alone
taking it.

## mod kind
One file a nushell error type's family of kinds, the error model's tables.

## mod text
NUON text without nushell: the reader and the writer port nushell's own, and
the serde serializer and deserializer carry any type to and from a NuVal and
so to and from the text.

## mod ty
The nu type: `model` its enum, its fields, and a `.nutype` file; `text` its
reader and writer as nushell spells a type; `check` a value held to one;
`shape` nushell's shapes and types, behind `nushell`; `error` the type
model's own error, apart from NUON's, since a type's text and its check fail
in their own terms.

## mod tests
nushell is the types' reference: nu-parser reads each type text through a
definition's signature, the one public path to `parse_shape_name`, and
nu-cmd-lang's default context supplies its `def`.

## use crate::error::*
## use snafu::OptionExt
The conversions' error context, behind `nushell`: `context` on an `Option`
for a range's second value overflowing and a row index past the platform's
usize. snafu itself is in every build, since the NUON error derives with it.

## mod nu
nushell's items the conversions name, under one path, so a reader tells them
from nuval's own, behind `nushell`, its `SyntaxShape` and `Type` among them
for the types. nushell's I/O error kind is `IoKind` here, apart from nuval's
`IoErrorKind`.

## use sourcetrait_common::cereal
cereal comes through the workspace's `sourcetrait_common`, its facade feature
set, rather than `sourcetrait_cereal` alone (User). `cereal::derived` emits
absolute `::serde`, `::bitcode`, and `::rkyv` paths, so those three are the
crate's own dependencies, at the instances cereal uses; miette and chrono are
taken at nu-protocol's.

## pub use
The value, its error and result, the data each kind of value holds, every
family of error kinds, and `wide`; the NUON text's style, error, and serde
conversions; the type, its field, a `.nutype` file, and the type's error and
result; with `nushell`, the two conversion traits and the conversion's
error.
