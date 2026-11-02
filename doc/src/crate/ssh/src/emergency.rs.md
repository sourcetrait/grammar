# emergency.rs

The controller's policy over the operator's emergencies, the MCP's own policy
with the session's MCP where the MCP has its Channel: each emergency logged to
grammar-ssh's own `emergency.nuonl` under its cache home, the line and the path
the operator's, the MCP's (the operator's `log.rs`), then sent up the session,
where the MCP logs it again under the engine's alias and announces it.
grammar-ssh's log is removed with its cache home at its end (`controller.rs`).

## struct EmergencyPolicy
### fn respond
The log is written first and unconditionally, so the record never depends on a
session being attached; the send follows it, dropped when none is.

### fn append
Clones share one lock and take their appends one at a time, as the MCP's do.
