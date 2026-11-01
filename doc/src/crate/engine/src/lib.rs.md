# lib.rs

## mod nu
nushell's items the engine names, under one path, so a reader tells them from
the engine's own.

## use sourcetrait_common::cereal
The system's own data is cereal data, so the engine takes `::serde`,
`::bitcode`, and `::rkyv` as dependencies of its own, the paths
`cereal::derived` emits, as nuval does.

## use nutype
The contract's types are nutype's, as its values are nuval's; the indices are
written and read as NUON through nuval's serde support.

## use std::os::unix::fs::MetadataExt
Unix alone, as `std::os::unix` is: its one reader, `plugins.rs`'s
`changed_time`, takes the inode's ctime from it there, and on Windows reads
the change time through `GetFileInformationByHandleEx` instead.

## mod equip
The equips by concept, so no file holds them all: `store`, the repository, its
index, its paths, its locks, and the builtins' install; `lifecycle`, an equip's
new, scaffold, commit, install, uninstall, and check; `signature`, the signature
block and `interface help`; `validate`, the validator.

## pub use
What a host may name: the engine's start, its parameters, and its host; its system
and its handle; the stateless pool's executor and the stateful engine; the
definition's contract, the lint, and the parse engines; the contract's type check
and its text; the source templates; the plugin registry's read; the call cache's
paths; the equips' store, lifecycle, signatures, and validation; and the namepaths.
Everything else is the crate's own.

The engine's requests, commands, and answers, the two lanes, the equips' documents
and commit result, the builtins, the diagnostics with their cap and positions, and the
error are the protocol crate's, re-exported here so every caller of the engine names
them as it did.
