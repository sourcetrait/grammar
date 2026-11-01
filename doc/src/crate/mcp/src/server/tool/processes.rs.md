# processes.rs

## fn process_list
The engine's rows rendered as records: `source_nonce` and `path` are per-KIND and omitted
when absent, so an entry's shape says which tool produced it without a discriminant field.

`args` is what lets an agent match entries against its OWN send-set, which matters because this
command shows ALL in-flight work on the host - a subagent sees the primary's calls and vice
versa.

ENGINE-SCOPED. The rows are the routed engine's, the MCP's own or a worker's, so a call in
flight on one host never shows on another, and a one-shot CLI invocation, which starts an
engine of its own, reports nothing rather than reaching into another host.
