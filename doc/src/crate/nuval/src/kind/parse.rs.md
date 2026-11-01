# parse.rs

The kinds of `ParseError` and `ParseWarning`, one per variant, made and kept
as `shell.rs`'s are. Four parse variants declare no code, `InvalidLiteral`,
`LabeledError`, `LabeledErrorWithHelp`, and `RedirectingBuiltinCommand`, and
their kinds have none.
