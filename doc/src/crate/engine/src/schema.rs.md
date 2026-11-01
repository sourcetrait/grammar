# schema.rs

The contract's types: nutype's `NuType`, read from an `execute` definition's or a rig
`main`'s parsed signature, and the strict check that holds a value to it. A type is DATA,
cereal-derived and serializing as its text, so the rig index records each call's data and
result types as they are and a call is held to them without its rig's source being parsed
again. nutype converts nushell's shapes and types; the contract's own rules are a pass over
the result, kept here because they are the contract's and not the type model's.

The contract is nushell's own, User's ruling: an agent writes Nu signatures and NUON
values, so the contract has no second schema language, an agent's data crosses as NUON
with no JSON conversion, and there is no void. `nothing` is a type like any other, at the
top as below it.

NO FALLBACK ANYWHERE: a type is the grammar's or it is refused, by name.

## fn compact
The signature block's spelling: the type's parts with the separators' spaces dropped.
nutype's `to_inline` is the full spelling, so the two can only ever differ in
PRESENTATION, never in what they understand a type to be.

## fn signature_group
The one difference from the full spelling is at the top: a record collapses to its
fields, `<refs:list<string>>`, the common case, while a table or `nothing` is spelled
whole, `<table<...>>` or `<nothing>`. Nested types keep their full spelling, so an open
record reads `record<>` inside one.

Every call has a data and a result type, so every call line carries its two groups:
that is what makes a call self-identifying in the signature block, which states no
separator characters at all.

## const OPEN_AT_TOP
An open record or table at the top of `data` would take anything, so a call's data would
document nothing. It is also the scaffold's skeleton, `data: record<>`, so the same
refusal is what stops a stub from committing as if it were finished.

## fn data_typedef
The `data` type, read from its parsed `SyntaxShape`, which keeps `path` and `directory`
as the author declared them. At the top only `nothing`, or a record or table with fields;
below it the whole grammar, an OPEN record or table among it, empty fields being
nushell's own spelling of open. The top is held before conversion, so a scalar at the top
is refused as one whatever it holds.

## fn result_typedef
The return type, read from the signature's output `Type`. The same top rule, and an open
record or table refused at EVERY depth: a return declares what the body produces, and an
open type there would assert nothing.

## fn not_data
nutype refuses a closure, a block, and the parser's other shapes as types no value takes;
the contract names each as no data type.

## fn grammar
`any` is refused at every depth. A list of tables is refused in both readings: nushell
treats `table<C>` and `list<record<C>>` as one type, so `list<list<record<...>>>` is a list
of tables too and is caught by its inner list of records. The refusal names the fix, a
record holding each table.

## fn check_value / fn check_fields / fn scalar_conforms
The strict check NuNu's contract asks for, which nushell's own makes no attempt at: its
`compare_types` and `is_assignable_to` hold a record OPEN, taking one with an extra field
as a subtype, so an undeclared field passes at any depth, and it checks no output when a
command runs at all. This walk holds a value to its declared type itself: each record
carries exactly its declared fields, each declared one required and an undeclared one
refused, each table row likewise, a list's every item, and a oneof's first member that
conforms. An EMPTY field list is an open record or table, and the walk stops there: that
is the one place any field stands.

A scalar is read by the value's own VARIANT, so `int` refuses a float, `float` refuses an
int, and `number` takes either, as nushell's types do. `path` and `directory` values are
strings by then, and a `glob` read back from NUON is a string too, so each takes one.
nutype's own check holds a NuVal the same way; this one holds nushell's value, which a
call's data already is, so the data is never converted only to be checked.

Every refusal names its PATH from the root the caller gives, `data` or `result`, with a
`.field` per record and an `[index]` per row or item: what an agent needs to find the one
wrong value in a large table.

## fn typedef_text
nutype's inline spelling, for messages and the test harness: an open record renders
`record<>`, where nushell's own Display would print a bare `record`, which is no valid type
syntax.
