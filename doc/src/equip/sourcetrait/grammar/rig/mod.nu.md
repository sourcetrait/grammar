# rig/mod.nu

The grammar builtin's rig, called as `grammar:<module>:<call>`: the essentials,
one call a plugin command the MCP bundles, `eye dir`, `eye git`, and `soak dir`.
Each call passes its data through unchanged, so the plugin's own help is the
call's whole behaviour.

`soak dir` takes its values in an open `fill: record<>`, since a template names
whatever fields it likes; an open record stands nested in a call's data, never
at its top. It returns nothing, as the command does. The two `eye` calls return
their tree in `{tree}`, a call answering a record.
