# lint.rs

Two rule classes walk an agent's `execute` definition as a parsed AST. The lint
exists to teach passing values as data, so a violation is an agent-fixable
refusal before any eval dispatches or writes.

## const ALLOWLIST_PATH_PREFIXES
Location-fixed contract paths, which pass uniformly because they cannot be
passed as data in any meaningful sense.

`/run/` is absent on purpose: that one passes as `$XDG_RUNTIME_DIR` in data.

## const DENYLIST_EXTERNALS
Most of these have nushell builtins, so the fix is idiomatic nu; the rest pass
in data or go through a helper.

## const REGEX_RECEIVERS
The built-in signatures carrying a `--regex` flag, source-grep verified against
`nu-command/src/` at 0.116.1, the engine's version; nu-cmd-lang, nu-cmd-extra,
and the standard library declare none. A decl that gains `--regex` in a later
engine would false-positive on its pattern argument until the list is
re-audited at that bump. Plugin commands are not covered at all.

Two receivers still false-positive on a slash-led pattern: `find --regex` takes
its pattern as the flag's value, which the walker checks like any named value,
and `str replace --multiline` implies regex without the flag the skip looks
for.

## const PARSE_PATH_RECEIVERS
Decls whose positional 0 is a parse-time-const file or module path. Such a path
cannot pass in data - it resolves at parse time, before any data exists - so
flagging it would be a false positive by construction.

`use` and `overlay use` parse to a Call whether their module resolves or not:
nu 0.116.1 parses the arguments by the decl's signature and keeps the import
pattern as the call's parser info, which the walker never visits. So this list
is what keeps a module path from tripping, resolved or not, as it is for
`source` and `source-env`.

## fn lint_block
The walker over a block another parse produced, positions measured from that
text's start: execute.rs lints an `execute` definition on the parse its
contract check already made, the def call's body reached through its positional
block like any closure's.

A diagnostic's position is measured from the text's start in the span space
(`file_start`): the engine holds the standard library's files ahead of this
one, so a raw span lands past the text. The rules themselves read the AST's
values and never the text. The lint reports rules, not parse errors; the
contract reports those.

The full-shell engine is required, not the lang-only one: lang-only collapses
`str replace --regex` into an ExternalCall where the flag is indistinguishable
from a positional, and the regex skip then cannot fire.

## fn push_diagnostic
`LINT_VIOLATION_CAP` bounds the list at three and the fourth candidate is
dropped silently. The `ControlFlow::Break` is threaded with `?` through every
walker arm, which is what short-circuits the recursion rather than merely
discarding results. There is no `more` marker on the wire - the envelope
reports no internal-cap truncation.

## fn walk_interpolation
The parts before the first expression are joined and checked as one literal at
the first part's position: an external's bare word is split at its quotes, so
adjacent literal parts are the one literal they read as. A literal part after
an expression extends a value the definition computed, `$'($root)/x'`, and is
never checked; each expression part is walked, its own strings held to the
rule. nushell parses a directory or file path argument holding an expression as
a string interpolation and a glob as a glob interpolation, so both come here.

## fn check_path
The rule reads a string's leading literal alone, untrimmed: a whole literal, or
an interpolation's literals before its first expression. A literal led by a
space or a newline is text, ` / ` a separator.

A bare prefix, `/`, `~/`, `./`, or `../`, passes only as the whole of a
string's text (`whole_text`): a plain string literal, or an interpolation
holding no expression, reads as a separator or text, `str join '/'`. A
directory, file path, or glob argument is a path the parser typed, so `cd /`
keeps its refusal, and a bare prefix leading an interpolation, `$'~/($rest)'`,
anchors a path at the home directory.

Past that the rule is use-blind: a single-segment `/tmp` trips like a deep
path, a glob character earns no exemption, and a `str contains` marker or a
message whose leading literal starts like a path trips like a filesystem path,
because the alternative is inferring intent from a string, which cannot be done
reliably. Each string is its own, so `$root ++ '/x'` trips on `'/x'` where
`$'($root)/x'` and `$root | path join x` do not.

The `://` skip is what keeps a URL from reading as a path, and a leading `//`,
a comment marker such as `///` or a protocol-relative URL, names no path
either.

## fn check_external_head
Matches on the basename, so `^/usr/bin/awk` matches `awk` and separately trips
the path rule on the head literal. Both surface, which is correct: they are two
different violations that happen to share a token.

A variable-headed external (`^$data.cmd`) skips silently, because there is no
literal to judge.
