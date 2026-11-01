# shape.rs

## impl TryFrom<&nu::SyntaxShape> for NuType
## impl From<&NuType> for nu::SyntaxShape
A shape converts both ways unchanged: it is what a signature declares, `path`
and `directory` among it.

## impl TryFrom<&nu::Type> for NuType
## impl From<&NuType> for nu::Type
nushell's `Type` is the resolved form: `path` and `directory` become `string`,
and its unions merge members as nushell's `OneOf` collects them, so a type
crosses it as its `resolved` form, a union possibly narrower.
