# command.rs

The `interface` tool's commands as data: one table the parse registers, the
description renders, and `--deny` names, so the three can never disagree. It
sits in the protocol crate so a frontend renders its tool description and parses
its deny set from the same catalog the runtime dispatches by, with no nushell in
it; the parse's decls are built from it where the parse runs. The commands
convert the tools that `execute`, `rerun`, and `call` left behind, each keeping
its tool's parameters and result as they were, `info` alone dropping its
purviews. The names are User's: `interface <topic> <action>`, or
`interface <action>` for `info` and `help`, with no underscore in any of them.

## const HELP_WORDS
`interface help interface` answers with the commands' signatures and `interface
help grmr` with grmr's: both words are reserved before any namepath is read.

## enum ArgShape
Three shapes cover every argument the commands take: a string, a list of
strings, and a whole number. Anything richer would be a value the agent could
build with an expression, and the contract takes literals alone.

## struct CommandSpec
`result` is the result's kind alone, `record`, `table`, or `nothing`, as the
description shows it (User): the commands answer what their tools answered,
built by the same serde structs, so the shape is the envelope's to carry and no
second copy of it is kept here.

`summary` is a summary's length, 80 characters at most (User), which a unit test
holds; User's own summaries stand where his draft has them.

### fn words
The deny name: what follows `interface `. A command is denied by the words an
agent types after the head, never by an invented key.

## const ENGINE_SET_FLAGS
Each field of an engine's entry a flag of its own, every one optional, so a set
changes what it names alone: the entry's model and nothing else (User), an
import being a command of its own. The known hosts take two flags, a list of
keys and a file, a flag holding one shape; each optional field's summary names
its default, and the platform's, having none, says a session needs it.

## const ENGINE_IMPORT_FLAGS
The ssh configuration's host and the file it lives in, each defaulting as
`ssh` itself does: the engine's alias, and the user's configuration.

## const COMMANDS
Ordered as the description lists them, grouped by topic. A topic's commands sit
together because `render_signatures` opens a heading when the topic changes.
`channel sync`, and `channel open`'s `mode`, are User's additions beyond the
tools converted; `sync` sits after `open`, in the order of the handshake it
serves. The remote engine commands, User's, sit under the Remote topic after
the remote channels', the same six verbs and `import`. The chat's two, User's,
take a topic of their own after Remote; `chat read`'s `page` is the catalog's
one optional positional, 0 when absent.

## fn with_head
A subcommand sent without its head, `info` for `interface info` or `dbg {a: 1}`
for `grmr dbg {a: 1}`, parses whole with the head prepended, quietly (User).
The head is taken only as a whole word, so `interfaces` gains one. The length
it added maps a position in the parsed text back to the text as it was sent.
It sits beside the catalog because the interface's parse and the engine's bed
parse both take it.

## fn render_command_list
The tool's description's list, User's format: `# Command Signatures`, a `##`
heading a topic, and a `- ` line a subcommand, its words whole, the
`interface` word left off as the end user leaves it (User). The signatures
are `interface help interface`'s. `denied` takes a command's words, so the
caller's deny set stays its own.

## fn render_signatures
`interface help interface`'s block, and the grammar-mcp skill's last section,
which `tool_docs` holds to it. Generated from the same table the parse
registers, so the documentation can't drift from what parses, and a denied
command leaves both at once.

User's format: under each topic's heading two lines a command, its summary and
its signature in backticks, with no bullet, no `def`, and the subcommand's
words alone, what the end user sends (User).
