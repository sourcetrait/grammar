# bed.rs

A bed call's rules, the interface tool's (User): one command, an embedded one,
its arguments literals, with no pipe, no other command, and no plugin. They
are held here, in the engine that runs the command, so a remote engine's
grammar-ssh holds its bed calls to them as the MCP's engine does.

## const KIND_BED
`bed::invalid`, every refusal's kind, as `interface::invalid` is the
interface's, my lean on errors, User's to veto as the tool is used (User).

## const EMBEDDED
The engine's embedded commands outside `interface`, `grmr` alone today (User);
a bed call's `command` is one of them or refused before any parse.

## fn analyze_bed
The subcommand, its command's word prepended where it doesn't open with it,
quietly, parsed by the lint engine, which holds every command an eval holds, so
a `grmr` command's arguments are checked by its own signature. A parse error is
refused before anything else is read, the rows deduplicated and capped as a
definition's are. Each position is measured from the parsed text's start in
span space (`file_start`), then back into the subcommand as sent. Its answer is
the text the eval runs, the whole command.

## fn held_call
The rules, made structural: one statement, one pipeline element, no
redirection, a call whose decl is the embedded command's, `grmr <action>`, and
every argument a literal. Any other head is refused here, an external among
them, before anything runs.

## fn literal
A literal is a scalar, a date, a value with its unit, or a list, a record, or a
table of literals, with no spread: the values an agent writes as data, never a
variable, a subexpression, or a closure. A full cell path with no members is
its head, the form nushell's parser gives a record argument.
