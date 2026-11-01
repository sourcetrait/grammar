# execute.rs

The execute contract, User's `include/human/mcp/execute.md`: an `execute` call's definition is exactly
one top-level `def execute` with an infix signature, and it is held to that before
anything is built or dispatched, so a refusal costs one parse and no thread. A call's
contract is execute's for its data, timeout, input, and return, User's ruling, so a rig's
`main` is held to the same signature check at commit.

## struct ExecuteDef
What the rest of a call needs from its definition and nothing more: the lane, the two
types the data and the result are checked against, and whether the source passes the
timeout. The types are schema.rs's `NuTypedef`, which DataCheck and ResultCheck walk.

## fn analyze_definition
ONE PARSE serves the contract and the lint, on the lint engine, which is the shell the
call will run on: the contract reads the registered decl's signature and the def call's
AST, and the lint walks the same block, so the two can never disagree about the text.

The parse is clean first. A definition that doesn't parse has no signature to judge, and
its errors are reported at their positions in the definition, in nushell's words through
nuval's `NuValError::short`.
A module-resolution cycle is checked before them, as the eval and the rig validator
check it, and answers `module::circular_import` alone: a cycle's own parse errors are
unstable and useless, and the definition's parse is now where a cycle is met first.

THE STRUCTURE is one top-level pipeline element, a call to `def` named `execute`.
Comments make no pipeline, so they stand. `export def`, an attribute block, a second def
for a helper, or a statement after the def each make a second element or another head,
and are refused: helpers and imports go inside the body.

THE LANE is the def's own `--env` flag, the user-facing switch the contract names for
stateful execution; nushell's `def --env` is exactly what lets the body's `$env` and `cd`
reach the caller, which the stateful lane then merges into its engine. `--wrapped` is
refused: it would admit unknown flags the contract has no place for.

THE SIGNATURE comes from the registered decl rather than the AST, since nushell has
already resolved defaults, optionality, and shapes there.

Violations are gathered where they are independent and capped at the lint's three, so
an agent fixing a definition sees its several faults in one round. The lint runs only on
a definition that holds to the contract, and is skipped when `lint` is false: a rerun
re-analyzes a definition the host cached after linting it.

## struct SignatureContract
The part of the contract a signature settles alone, shared by a definition and a rig's
`main`; the lane and the lint are execute's own. A call's main is free of the lint's
restrictions, User's ruling, so it may hard-code a path or run a denied command.

## fn contract_signature
`data` is the one required positional; `timeout?` the one optional, typed `duration`, its
default refused because the host always passes the call's timeout and a default would
only mislead. nushell adds `--help` to every custom command, the one flag allowed. An
absent infix signature reads as `any -> any` and is refused with the input check.

`data`'s type is read from its `SyntaxShape`, which keeps `path`, `directory`, and an
open record apart; the return type from the output `Type`, which folds `path` and
`directory` into `string`, a difference a value check cannot see anyway.

Each refusal names its command, `execute` or `main`, so an author reads the one held.
Every violation is gathered rather than the first returned, as the definition's are.
