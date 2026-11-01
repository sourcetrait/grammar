# lib.rs

nutype does for nu types what nuval does for values (User): the type model as
cereal data, so a schema crosses the wire in bitcode or rkyv; its text, read
and written as nushell spells types, so a `.nutype` file is its schema; and
nushell's own shapes and types converted both ways. Any crate that wants nu
type definitions takes it.

## feature nushell
nushell's shapes and types take nu-protocol and nuval's own `nushell`. The
feature is on by default and gates every nushell dependency (User); the
workspace takes nutype with default features off, and the model, its text, and
its check build without.

## mod tests
nushell is the reference: nu-parser reads each type text through a
definition's signature, the one public path to `parse_shape_name`, and
nu-cmd-lang's default context supplies its `def`.
