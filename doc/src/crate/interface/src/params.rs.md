# params.rs

Each command's parameters, as the tools that became the commands took them, so a
controller calls a command's logic with the very value its tool used to
deserialize. They serialize for the nonce a call mints over them.

## struct ConfigChannelParams
A partial update - supply only what should move - so a caller adjusting one
threshold does not have to restate the other three and risk reverting a change
it did not make.

Windows are integer seconds because the interface's arguments are strings and
whole numbers alone, literals the contract reads without evaluating
(`contract.rs`), so a nu `duration` has no shape there. The `10sec` form lives
in the model and the docs, never in a command.

## struct EquipParams
One parameter for all four commands, the source path: the namepath is read off
it, so the equip is wherever its source says it is.
