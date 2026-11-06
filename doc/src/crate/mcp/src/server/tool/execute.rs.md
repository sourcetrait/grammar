# execute.rs

The `execute` tool, User's `grammar/execute`: one definition evaluated against NUON
data. The contract is User's `include/human/mcp/execute.md`, its parameters his
`doc/human/tools.md`, and the engine holds a call to it (the engine's `serve.rs`).

## struct ExecuteParams
Four strings, nothing structured: the host an alias, the definition Nu source, the data
NUON text, the timeout a Nu duration's text. The agent writes Nu and NUON, never a JSON
schema; the definition's own signature is the schema.

## fn execute
The description is User's `include/human/mcp/execute.md`, verbatim through
`include_str!`: rmcp's tool macro takes a doc attribute as the description when no
`description` is given, a macro call included, so the agent reads his contract as he
wrote it. The doc lint's list-indent rule is allowed on this one fn: the text is written
for the agent, not for rustdoc, and it stays as he wrote it.

The nonce is minted FIRST, over the parameters, so every refusal carries it; the host
is routed next (`NuSh::route`, `common.rs`). The definition is
held to the contract and the lint at the edge, its refusals positioned in the agent's
text, and taken as its model, its signature's shapes beside its body's text (the
engine's `DefineRequest`); the data and the timeout are NUON, so each is taken at the
edge as the value it names (the engine's `nuon_data` and `nuon_timeout`), a text that
isn't NUON refused here as `data::invalid` or `timeout::invalid`. The engine that runs
the call, the MCP's or grammar-ssh's, writes the definition from the model and holds the
data and the timeout to it.
