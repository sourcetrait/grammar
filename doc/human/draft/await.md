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
        - `job: record<pid: int, started: timestamp>` Provided by the engine
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
- Engine default: `[ [prepare, execute, status, stop]; [120sec, 120sec, 120sec, 120sec] ]`
#### Parameter: `heartbeats`
- Intervals at which the engine:
  - Checks to see if the process is still running
  - Runs the contractual `status` and relays it to the engine Channel
- Last interval in list is repeated until the job ends
- Minimum of two intervals if specified
- Engine default: ``[ 30sec, 60sec, 5min, 15min, 30min, 45min, 60min ]``

## Example
Tool parameter `module`:
```nu
export def prepare [data: record<dir: directory>, timeouts?: record<prepare: oneof<nothing,duration>, execute: oneof<nothing,duration>, status: oneof<nothing,duration>, stop: oneof<nothing,duration>>, heartbeats?: list<duration>]: nothing -> table<files: path> {
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
Tool parameter `data`:
```nuon
[[num]; [5], [3], [2]]
```
Tool parameter `timeouts`:
```nuon
[ [prepare, execute, status, stop]; [null, 0ns, null, null] ]
```
Tool parameter `heartbeats`:
```nuon
[ 30sec, 90sec, 5min, 30min ]
```
Tool result:
```nuon
{nonce: "5zNe1n9JO9G", result: [[id, name]; [0, zero], [1, "10"]]}
```

