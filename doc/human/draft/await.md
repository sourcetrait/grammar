# await
> DRAFT: Do not implement

## initial idea
 tentatively, the tool could be called await. parameters to the tool: module and data
  my initial idea: similar to how we have a contract in 'execute' for a single definition, we would have a contract for an entire module (module field) that requires four contractual exported defs (and no other exports, but whatever else may be needed is allowed):
  - prepare [data: nothing/record<PREP_SCHEMA>/table<PREP_SCHEMA>, timeouts?: record<prepare, execute, status, cancel>, heartbeats?: list<duration>] -> result<EXEC_SCHEMA> # prepare the data field that will be passed to execute, using data passed. a pipeline that allows last-minute runtime initialization. the initial schema and the result schema are separate. it can simply be a pass-thru { $data } if nothing further is to be done. interval is a list of durations when status should be called, from start time, with the last being the one that is re-used after each listed step (eg, [30sec, 60sec, 5min, 30min])
  - execute [data<EXEC_SCHEMA>, timeout?(from prep)] -> result<SCHEMA> # ran by the engine with the prepared data by launching it in a background process
  - status [background: record<BG_SCHEMA>, data<EXEC_SCHEMA>, timeout?(from prep)] -> result<STATUS_SCHEMA> # ran at intervals (default or provided to prepare) as a heart beat check
  - stop [background: record<BG_SCHEMA>, data<EXEC_SCHEMA>, timeout?(from prep)] -> result<CANCEL_SCHEMA> # tears down the process, attempting to do it cleanly

  for execute (await) only: a timeout of (>)=1000wk (~19 years) will disable timeout checking for execute
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
