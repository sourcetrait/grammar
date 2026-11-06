# config.rs

The record is the host's, mirroring the TOML, the engine's controller answering it
to an ask (`HostAsk::Config`); these decls list its keys, read settings by them, and
pin through the host (`HostAsk::Pin`).

## const KEY_SEP
A dotted key is a path into the record, a table's field,
`supervisor.cpu_warn_fraction`: the one naming scheme `config list` gives and
`config get` and `config pin` take.

## fn config_record
One ask a read, so a decl lists and walks the one record it was answered, and what
it reports is what is in force at that moment.

## fn config_keys
Derived from the record rather than restated beside it, so the keys `config list`
names and an error message lists cannot drift from the keys the record has. A table
the config leaves null, `identity` on a host whose file names none, is a key of its
own, reading null.

## fn unknown_key
The title names the problem and the label the vocabulary, nushell's split.
`NuValError::short` carries both into the envelope's message, so the keys in the
title as well read twice. Listing the valid keys is the value of this error, and a
test asserts the list reaches the message once.

## fn value_type
Spelled as a union rather than `any`, because the set is closed and naming it lets
a caller's own annotation be strict too.

## fn settings_type
`config get` answers a table, each key asked beside its value in the order asked: a
list of keys in, a row a key out, so `config get (config list)` reads every setting
and one key's value reads by its row, `get 0.value`. A record keyed by the dotted
keys would take a cell path whose members hold dots, which `get` reads as one
column's name.

## struct GrmrConfigGet

### fn run
An unknown key is an error rather than a null, so a typo cannot read as
"configured to nothing". The two are indistinguishable at the call site and only
one of them is a bug the caller wants to hear about. The call fails at its first
unknown key, answering no partial table.

## struct GrmrConfigList
The keys in the record's order, its tables' and each table's fields', so the list
reads as the TOML does.

## struct GrmrConfigPin
Only the `[supervisor]` warning lines are pinnable. `[channel]` is excluded so
`interface channel config` stays the sole mutator there - a value with two mutators
is one whose effective setting depends on which surface you happen to ask.
Everything else in the config is not a runtime value at all.

A pin the host turns down is titled `grmr config pin failed`, its reason the label,
as the sends' refusals are.
