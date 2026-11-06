Executes the provided Nu language module definitions as a Nushell job
(background process) suite.

The module contract requires definition exports for:
- `prepare` Initialization; Before a background process is launched
- `execute` Execution; Within a job started by the engine
- `status` Heartbeat; Periodic reports via Channel
- `stop` Interruption; Tears down a job before it ends normally

## Language Contract
This contract is aggressively enforced and will error if broken.
### General
- One module with the four required defintions and any amount of other
syntax for internal use.
#### Contractual Definitions
- The infix `input` type must always be `nothing` for contractual
definitions.
- Values must be passed as data (refer to the `execute` tool's Lint)
##### Parameter: `data`
- Typed as:
  - `nothing`
  - `record<TYPEDEF>` with a strict type definition
  - `table<TYPEDEF>` with a strict type definition
- Denies untyped `record<>` or `table<>` at its top level
  - Nested fields allow this
- Denies undeclared fields
  - Nested untyped `record<>` and `table<>` allow this
- Denies lists of tables
##### Parameter: `timeout`
- Typed as `duration`
- Specific for its conractual definition, as supplied to the `timeout` record
  on `prepare`
- Must be either optional (`timeout?`) or omitted from signature
- Value is inspected internally for tool timeouts; Default: `120sec`
- Denies a default value set by signature

### Definition: `prepare`
#### Allowed Parameters
##### Required: `data`
- Typed as:
  - `nothing`
  - `record<TYPEDEF>` with a strict type definition
  - `table<TYPEDEF>` with a strict type definition
- Denies untyped `record<>` or `table<>` at its top level
  - Nested fields allow this
- Denies undeclared fields
  - Nested untyped `record<>` and `table<>` allow this
- Denies lists of tables


  - prepare [data: nothing/record<PREP_SCHEMA>/table<PREP_SCHEMA>, timeouts?: record<prepare, execute, status, cancel>, heartbeats?: list<duration>] -> result<EXEC_SCHEMA> # prepare the data field that will be passed to execute, using data passed. a pipeline that allows last-minute runtime initialization. the initial schema and the result schema are separate. it can simply be a pass-thru { $data } if nothing further is to be done. interval is a list of durations when status should be called, from start time, with the last being the one that is re-used after each listed step (eg, [30sec, 60sec, 5min, 30min])
  - execute [data<EXEC_SCHEMA>, timeout?(from prep)] -> result<SCHEMA> # ran by the engine with the prepared data by launching it in a background process
  - status [background: record<BG_SCHEMA>, data<EXEC_SCHEMA>, timeout?(from prep)] -> result<STATUS_SCHEMA> # ran at intervals (default or provided to prepare) as a heart beat check
  - stop [background: record<BG_SCHEMA>, data<EXEC_SCHEMA>, timeout?(from prep)] -> result<CANCEL_SCHEMA> # tears down the process, attempting to do it cleanly

# ref-execute.md

## Language Contract
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


# idea-await.md
> DRAFT: Do not implement

## initial idea
 tentatively, the tool could be called await. parameters to the tool: module and data
  my initial idea: similar to how we have a contract in 'execute' for a single definition, we would have a contract for an entire module (module field) that requires four contractual exported defs (and no other exports, but whatever else may be needed is allowed):
  - prepare [data: nothing/record<PREP_SCHEMA>/table<PREP_SCHEMA>, timeouts?: record<prepare, execute, status, cancel>, heartbeats?: list<duration>] -> result<EXEC_SCHEMA> # prepare the data field that will be passed to execute, using data passed. a pipeline that allows last-minute runtime initialization. the initial schema and the result schema are separate. it can simply be a pass-thru { $data } if nothing further is to be done. interval is a list of durations when status should be called, from start time, with the last being the one that is re-used after each listed step (eg, [30sec, 60sec, 5min, 30min])
  - execute [data<EXEC_SCHEMA>, timeout?(from prep)] -> result<SCHEMA> # ran by the engine with the prepared data by launching it in a background process
  - status [background: record<BG_SCHEMA>, data<EXEC_SCHEMA>, timeout?(from prep)] -> result<STATUS_SCHEMA> # ran at intervals (default or provided to prepare) as a heart beat check
  - stop [background: record<BG_SCHEMA>, data<EXEC_SCHEMA>, timeout?(from prep)] -> result<CANCEL_SCHEMA> # tears down the process, attempting to do it cleanly

  for execute (await) only: a timeout of 0ns (aka explicitly indefinite) will disable timeout checking for execute
  would this cover what we currently using the long runner for?

## example
```nu
export def prepare [data: record<dir: directory>, timeouts?: record<prepare: duration, execute: duration, status: duration, cancel: duration>, heartbeats?: list<duration>]: nothing -> table<files: path> {
    ls ($data.dir | path join '*.md' | into glob) | select name
}

export def execute[data: table<files: path>, timeout?: duration]: nothing -> record<num: int> {
    {
        num: $data.files | length
    }
}

export def status[background: record<pid: int, started: timestamp>, data: table<files: path>, timeout?: duration]: nothing -> record<nothing> {
}

export def stop[background: record<pid: int, started: timestmap>, data: table<files: ppath>, timeout?: duration]: nothing -> record<nothing> {
}
 ```

