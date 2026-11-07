# await_tool.rs

The `await` tool, its description `include/mcp/tool/await.md` through the handler's doc
attribute, as `execute`'s is. The file and its method take other names, `await` being
Rust's keyword; the tool's own name is `await`.

## struct AwaitParams
Every parameter a string, as each tool's is: the host, the module's Nu text, prepare's
data as NUON text, and the timeouts and the heartbeats as NUON text, each absent for the
engine's defaults.

## fn await_suite
The edge takes the three NUON parameters as their values, each refused under its own
kind, and sends the module on as its text: the module is Nu, so it crosses as a string,
and the serving engine holds it to the suite's contract (User). The answer is the
PREPARED record; the job's events follow on the Channel.
