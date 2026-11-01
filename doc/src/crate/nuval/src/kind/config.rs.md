# config.rs

The kinds of `ConfigError` and `ConfigWarning`, one per variant, made and kept
as `shell.rs`'s are.

## fn ConfigErrorKind::of
None for `ConfigError::ShellError`, whose shell error takes its own kind.
