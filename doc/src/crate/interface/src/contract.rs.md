# contract.rs

The `interface` tool's command held to its contract before anything runs: one
call of one interface command, unpiped, its arguments literals. User's two
rules are the contract's spine: the context has nothing else it can access, and
the agent can't pipe commands.

## enum InterfaceCommand
The routing table: a command grouped by the system that owns it, the Channel's
five, the remote channels' seven, the remote engines' seven, and the engine's
fourteen, so a controller routes the group and the group's own commands stay
together. Each variant carries its tool's parameters struct as it stood
(`params.rs`), a new command its own.

## enum RemoteEngineCommand
The remote engines' commands: an entry in the MCP's remotes file and the MCP's
session with the engine, so a controller runs them on itself whatever the
tool's host.

## enum EngineInterfaceCommand
Info, help, the processes, the purviews, and the equips: the engine's to answer,
the purviews' and `info`'s and `help`'s with the session's view, which only a
controller holds.

## fn parse_command
nushell's own parser over the interface engine, so the arguments' types are
checked by the commands' signatures, as any Nu call's are, and an error lands
at the agent's line and column, measured from the command's start in the span
space (`get_span_for_filename`, as the engine's `file_start` reads it), since a
working set numbers spans after whatever the engine holds.

A parse error is refused before anything else is read: a refused parse's
expressions are garbage. The rows are deduplicated and capped as a definition's
are, by the protocol's `LINT_VIOLATION_CAP`, so the client caps them with no
engine.

## fn held_call
The two rules, made structural. One statement, one pipeline element, no
redirection: so no `;`, no newline-separated second command, no `|`, and no
`o>`. The element must be a call to an interface decl; with nothing else in the
engine, any other head parses as an external, which is refused here rather than
ever reaching a process.

## fn literal
The arguments are read off the AST, never evaluated: a string, a raw string, an
int, or a list of strings. A variable, `$nu` and `$env` among them, a cell path,
a subexpression, an interpolation, or a spread is refused, even where the
signature's type check would have let it through, because evaluating it is
access the contract withholds. `eval_constant` is deliberately not used: a
constant can read `$nu`.

## fn count
The tools took unsigned counts; a literal int is signed, so the range is checked
here, a refusal naming the flag.

## fn text / fn texts
A required argument's absence is the parse's to refuse, so these default rather
than fail.

## fn optional_text / fn optional_texts
An optional flag stays absent when not given, so a set changes only what it
names. An empty list stays a list, so `--host-keys []` reaches the entry's
check and is refused there rather than read as absent.

## fn mode
The parse can hold a string only as a string, so the two modes are held here, an
unknown one refused at its position as any argument's refusal is, naming the
two; nothing reaches the channel but a mode it knows.
