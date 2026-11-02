# lib.rs

## use sourcetrait_common::cereal
The operator's protocol with its manager is cereal data, so the operator takes
`::serde`, `::bitcode`, and `::rkyv` as dependencies of its own, the paths
`cereal::derived` emits, as the engine does.

## use sourcetrait_grammar_protocol
The engine's health, its hung evals, and its lanes, the portable definitions the
operator reads the engine by without depending on it.

## use nuval
The emergency log's lines are NUON text, written through nuval, which the
operator takes without nushell's feature, as the protocol does.

## mod tk
tokio's items the system's loop names: its `select!`, its interval, the probes' blocking
threads, and the channel they answer on.

## pub use
What a manager names: the system, its handle, and its parameters and paths; the
protocol, the observation it passes and the emergency it gets back, with every
emergency's struct, the protocol crate's re-exported; the emergency log's line,
its announced record, its path, and its append, which both hosts write
(`log.rs`); the supervisor lines and their range check; the pins, which the
host's evals pin through; a process's start time, the identity the pins and the host's
scratch directories share; the subreaper; and the interval the manager observes at.
Everything else is the crate's own.
