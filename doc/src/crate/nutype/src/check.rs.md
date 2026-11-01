# check.rs

## impl NuType
### fn check
A record or a table holds exactly its declared fields, as the engine's contract
does, nushell's own checks holding a record open; an open record or table
takes any. A value is read by its variant, so `int` refuses a float and
`number` takes either; `path`, `directory`, and `glob` take strings, since
that is what NUON reads them back as.
