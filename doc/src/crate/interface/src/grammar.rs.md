# grammar.rs

The interface commands where the parse runs: the protocol crate's catalog made
into decls an isolated engine registers.

## fn signature
Optional positionals stay optional in the parse; `config`'s thresholds are flags
because four optional positionals would make setting the last one mean passing
nulls for the rest, and the remote commands' optional fields are flags for the
same reason. The category is cosmetic: the decl is never listed anywhere but the
description.

## struct InterfaceDecl
A signature for the parser and nothing more. The command is parsed, never
evaluated: dispatch is native, so `run` refuses, and reaching it would mean
something evaluated the interface engine, which nothing does.

## fn interface_engine
`EngineState::new()` holds no command at all, not even the language's keywords,
so the decls added here are the whole of what a command can name. That is the
isolation User asked for: no `use`, no `source`, no `let`, no external that
resolves to anything but a refusal. A controller builds it once at startup from
its deny list, fixed by then, `denied` naming a command by its words, as the
protocol's renders of the catalog take it.
