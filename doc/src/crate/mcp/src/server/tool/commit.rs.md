# commit.rs

## fn equip_commit
The commit is the routed engine's `EquipCommit` (the engine's `serve.rs`): a builtin's bare
name resolved, the equip's write lock taken, the source validated on a current engine. The
tool answers the commit's own result, its changed paths and the validator's `warnings`. A
remote engine commits into its own namespace, so its calls reach the rig and the MCP's
don't.
