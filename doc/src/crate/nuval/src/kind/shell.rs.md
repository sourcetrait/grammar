# shell.rs

The kinds of `ShellError` and `ShellWarning`, one per variant. The tables were
generated once from nu-protocol's own attributes: each variant's code from its
`#[diagnostic(code(...))]`, and its message from its `#[error(...)]` headline,
verbatim where the headline holds no field and in a field-free form of it where
it does. They are kept by hand since: the match from nushell's type is
exhaustive, so an engine upgrade that adds a variant fails the build, and its
code and headline are copied from its attributes the same way.

## enum ShellErrorKind
Every variant but the six wrapping another type, `Network`, `Io`, `Generic`,
`LabeledError`, `Job`, and `ChainedError`, which take their own families. The
unix-only variants are kinds on every platform, so an error made on one
deserializes on another; only their match arms are unix-only.

## fn ShellErrorKind::of
None for a wrapping variant, so the model dispatches it by its inner type.
Deprecated variants are matched as any other while nushell still has them.
