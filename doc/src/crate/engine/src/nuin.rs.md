# nuin.rs

## struct SignatureDef

A grimm command's signature as static data, so its name, summary,
category, and examples are consts beside its implementation, and
`SignatureTrait::vocab` applies them to a nushell `Signature`.

## trait SignatureCategoryTrait

`root` names the custom category every grimm command shares; `str` is the
command's own category, carried as its search term.
