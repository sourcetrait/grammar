# io.rs

## enum IoErrorKind
nushell's own I/O error kinds, a standard library kind among them. Its codes
and messages are nushell's, from `IoError`'s Diagnostic and Display.

## enum StdIoErrorKind
Every stable `std::io::ErrorKind`, `Uncategorized` for any other, since the
standard library's enum is non-exhaustive. nushell names the code of twenty
kinds and derives the rest from the kind's description, lowercased with
underscores, so `ReadOnlyFilesystem`'s code keeps its hyphen; a message is
the description with its first letter raised, `NotFound`'s nushell's own.
A unit test holds every kind's code and message to nushell's.
