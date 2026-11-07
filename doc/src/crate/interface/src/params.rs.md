# params.rs

Each command's parameters, as the tools that became the commands took them, so a
controller calls a command's logic with the very value its tool used to
deserialize. They serialize for the nonce a call mints over them.

## struct ConfigChannelParams
A partial update - supply only what should move - so a caller adjusting one
threshold does not have to restate the other three and risk reverting a change
it did not make.

Windows are integer seconds, whole numbers the contract reads off the AST
(`contract.rs`). The `10sec` form lives in the model and the docs, never in
this command.

## struct RemoteEngineSetParams
Every field optional but the alias, a set naming only what it changes; the
known hosts come as a list or a file, two flags for one field, since a flag
takes one shape, and either replaces the entry's known hosts whole.

## struct RemoteEngineImportParams
`from` defaults to the engine's alias, so a host named alike in the ssh
configuration imports by the alias alone; `ssh_config` names a file read in
place of the user's, as `ssh -F` does.

## struct EquipParams
One parameter for all four commands, the source path: the namepath is read off
it, so the equip is wherever its source says it is.

## struct AwaitRigParams
No tool became `await rig`, so it serializes for no nonce: the interface call's
own nonce is minted over the subcommand. Its values are the subcommand's
literals as nuval carries them, the shapes the `await` tool takes as NUON, so
the engine holds them as it holds the tool's. An absent optional stays absent,
which prepare takes as null.

## struct ChatSendParams / struct ChatReadParams
New with the chat, so no tool came before them. A send names its peer by the
nom the remote channel list shows, never an alias, as every send to a peer does;
a read's page is 0 where the command names none, so a short chat reads with its
id alone.
