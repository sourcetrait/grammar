# cork/error.rs

## enum CorkError
Messages are plain text: an agent reads them through the MCP, where a
terminal's color codes are noise.

## impl ErrorTrait for CorkError
A path error becomes nushell's own I/O diagnostic, so its label sits on the
path argument when the error carries that argument's span.
