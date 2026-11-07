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
- One module with the four required definitions and any amount of other
  syntax for internal use.
#### Contractual Definitions
- The infix `input` type must always be `nothing` for contractual
  definitions.
- Values must be passed as data (refer to the `execute` tool's Lint contract)
##### Parameter: `data`
- Same contract as the `return` for the `execute` tool, with exceptions below
- Typed as:
  - `nothing`
  - `record<TYPEDEF(INITIAL|PREPARED)>` with a strict type definition
  - `table<TYPEDEF(INITIAL|PREPARED)>` with a strict type definition
  - TYPEDEF here is split between two forms throughout the contractual definitions:
    - `INITIAL` Specific to `prepare`. Received directly from caller
    - `PREPARED` Common to: `execute`,`status`,`stop`
      - Typed as a `record` with two fields:
        - `job: record<id: int, started: datetime>` Provided by the engine
        - `prepared: PREPARED_RESULT_TYPE` The result type defined for `prepare`
##### Parameter: `timeout`
- Same contract as the `timeout` for the `execute` tool, with exceptions below
- Value corresponds to its named field within the `timeouts` record for `prepare`
- Specific to `execute`: `timeout` of `0ns` is allowed and disables timeout
  checking for the job
### `return` Type
- Same contract as the `return` for the `execute` tool.
### Definition: `prepare`
- Directly receives the tool parameters: `data`,`timeouts?`,`heartbeats?`.
#### Parameter: `timeouts`
- Each record field provides a timeout for its corresponding contractual definition
- Value of `null` will use engine defaults
- Only `execute` allows for `0ns` to disable timeouts for the job itself
- Engine default: `{prepare: 120sec, execute: 120sec, status: 120sec, stop: 120sec}`
#### Parameter: `heartbeats`
- Intervals at which the engine:
  - Checks to see if the process is still running
  - Runs the contractual `status` and relays it to the engine Channel
- Last interval in list is repeated until the job ends
- Minimum of two intervals if specified
- Engine default: 30sec, 90sec, 5min, 15min x4, 20min x6, 30min x4, 60min

## Operation
- The result is the `PREPARED` record, once the job starts.
- Packets, `await` the call's nonce: `mcp/await/Status` per heartbeat,
  `{await, status}`; `mcp/await/Result` at `execute`'s end, `{await,
  result}`; `mcp/await/Stop` after `stop`, `{await, reason, result}`; `error`
  replaces a failed value.
- `interface await stop <await>` stops the job.

## Example
Tool parameter `module`:
```nu
export def prepare [data: record<dir: directory>, timeouts?: record<prepare: oneof<nothing,duration>, execute: oneof<nothing,duration>, status: oneof<nothing,duration>, stop: oneof<nothing,duration>>, heartbeats?: list<duration>]: nothing -> table<file: string> {
    glob ($data.dir | path join '*.md') | wrap file
}

export def execute [data: record<job: record<id: int, started: datetime>, prepared: table<file: string>>, timeout?: duration]: nothing -> record<num: int> {
    {num: ($data.prepared | length)}
}

export def status [data: record<job: record<id: int, started: datetime>, prepared: table<file: string>>]: nothing -> record<at: datetime> {
    {at: (date now)}
}

export def stop [data: record<job: record<id: int, started: datetime>, prepared: table<file: string>>]: nothing -> nothing { }
```
Tool parameter `data`:
```nuon
{dir: notes}
```
Tool parameter `timeouts`:
```nuon
{prepare: null, execute: 0ns, status: null, stop: null}
```
Tool result:
```nuon
{nonce: "5zNe1n9JO9G", result: {job: {id: 3, started: 2026-11-07T00:10:00+00:00}, prepared: [[file]; ["notes/a.md"]]}}
```
