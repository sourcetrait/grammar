# suite.rs

The await suite's contract, `include/mcp/tool/await.md`: one module whose four exports,
`prepare`, `execute`, `status`, and `stop`, each hold to the `execute` tool's contract
with the suite's exceptions, checked before anything runs, so a refusal costs one parse.
The document is User's draft, copied outside his rein with conservative edits; the code
holds to it as it stands.

## const SUITE_EXPORTS
The order a job runs them, which is also the order the timeouts record names them, so a
check of the record's fields and a check of the module's exports read one list.

## struct SuiteDef
What a job needs of its module and nothing more: prepare's data and return types, which
optional parameters prepare declares, and each other definition's return type and
whether it takes `timeout?`. Their data needs no field: it is the PREPARED record, made
from prepare's return type.

## fn prepared_record_type
The job's record beside prepare's value, the one data execute, status, and stop take. The
job's `id` is the jobs table's, as `job list` shows it; a job is a thread in the host's
process, holding no pid of its own.

## fn analyze_suite
The module parses inside a wrapper module, as the equip validator parses a rig's file, so
its exports are the wrapper's `decls` and nushell has resolved each signature. Positions
are the module's own: the wrapper's prefix is taken off the file's start.

THE EXPORTS are exactly the four. An extra export is named, so an author keeps a helper
by dropping its `export`; a module, a constant, an `export-env`, or a `main` exported is
refused, each being surface the job has no use for.

`--env` is the decl's block's `redirect_env`, refused: no definition of a suite runs on
the stateful lane, so an env change would reach nothing.

THE SIGNATURES: prepare's own check below, the other three by `contract_signature`, the
`execute` tool's, so their contract is execute's exactly (User). Each one's `data` is then
held to the PREPARED record by `same_shape`, since a declared `path` keeps its variant
where prepare's return type, read from nushell's output `Type`, has folded it to
`string`.

The lint walks the whole module, its helpers among it, as it walks an execute
definition's body; it runs only once the contract holds.

## fn prepare_contract
`data` is held under the return contract, no record or table open at any depth, as the
document reads. `timeouts?` and `heartbeats?` are positional, so their order is fixed: a
prepare may declare neither, the first, or both. Each declared type must be the
contract's, compared by shape, and neither takes a default, since the call fills them
as it got them.
