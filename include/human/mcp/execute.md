Executes the provided Nu language definition against the NUON `$data` provided.

## Language Contract
This contract is aggressively enforced and will error if broken.
### General
- One top-level definition named `execute` that uses a Nu infix type signature
- The only def-level flag allowed is `--env` (stateful).
- Only allowed parameters: `data` and `timeout`
  - `data` parameter is mandatory
- The infix `input` type must always be `nothing`
- Values must be passed as data (see Lint)
### `timeout` Type
- Typed as `duration`
- Must be either optional (`timeout?`) or omitted from signature
- Value is inspected internally for tool timeouts; Default: `120sec`
- Denies a default value set by signature
### `data` Type
- Typed as:
  - `nothing`
  - `record<TYPEDEF>` with a strict type definition
  - `table<TYPEDEF>` with a strict type definition
- Denies untyped `record<>` or `table<>` at its top level
  - Nested fields allow this
- Denies undeclared fields
  - Nested untyped `record<>` and `table<>` allow this
- Denies lists of tables
### `return` Type
- Infix return type typed as:
  - `nothing`
  - `record<TYPEDEF>` with a strict type definition
  - `table<TYPEDEF>` with a strict type definition
- Denies untyped `record<>` or `table<>`
- Denies undeclared fields
- Denies lists of tables
### Lint
- Denies hard-coded paths starting with: `/ ~/ ./ ../`
  - Allows URLs
  - Allows well-known system paths starting with: `/dev/ /etc/ /proc/ /sys/`
	- Allows use with: `use` `overlay use` `source` `source-env`
- Denies externals (`^<cmd>`) handled better with Nushell:
    awk bash cp date echo find grep jq ls nu rg rm sed sh sort which

## Operation

Execution can be either stateless (default) or stateful (`--env`).

Stateless operation runs across a pool of isolated stateless workers.

Stateful execution is queued against a single stateful worker and persists
environment changes across the worker's long-term context.

## Examples
Note: Nonces would be random.
### Stateless execution
Tool parameter `definition`:
```nu
def execute [data: table<num: int>, timeout?: duration]: nothing -> table<id: int, name: string> {
	let total = $data | get num | math sum
	[
		[id name];
		[0 zero]
		[1 ($total | into string)]
	]
}
```
Tool parameter `data`:
```nuon
[[num]; [5], [3], [2]]
```
Tool result:
```nuon
{nonce: "5zNe1n9JO9G", result: [[id, name]; [0, zero], [1, "10"]]}
```
### Stateful execution
Tool parameter `definition`:
```nu
def --env execute [data: record<dir: directory, file: path>, timeout?: duration]: nothing -> record<id: int, name: string> {
	cd $data.dir
	let kind = open --raw $data.file | from yaml | get kind  # hypothetical: value retrieved is 'foobar'
	let count = ls ($data.dir | path join '*.md' | into glob) | length
	$count | save -f ($data.file + '.count')
	{
		id: 3
		name: $kind
	}
}
```
Tool parameter `data`:
```nuon
{dir: /tmp, file: barn.dat}
```
Tool result:
```nuon
{nonce: aOiwlfCpNKg, result: {id: 3, name: foobar}}
```
