Grammar MCP tool calls primarily use parameters and results in the JSON-escaped
text form of:
- Nu
- NUON
- Namepath (`<author>/<name>:<module/path>:<function>`)
- Nonce (base62 pseudo-random xxh3-64 hash)

MCP tool calls require a `host` (alias) parameter - where the tool is run from.
Normal operation on localhost uses `local` for its value.

On success, the MCP JSON-RPC `result.content` will be populated with NUON text,
typed as:
```nu
record<
	nonce: string
	result: oneof<nothing, record<TYPEDEF>, table<TYPEDEF>>
>
```

On failure, the MCP JSON-RPC `result.content` will be populated with NUON text,
typed as:
```nu
record<
	nonce: string
	error: table<
		kind: string
		source: oneof<nothing, record<
			path: oneof<nothing, string>
			position: list<int>
		>>
		message: string
	>
>
```
