# operator.rs

## struct OperatorWatch
The operator's emergencies go to the controller's policy (`emergency.rs`),
which logs each and sends it to the session's MCP, as the MCP's own handler
gives its emergencies to its policy.

## fn engine_health, fn spawn_observer
Each interval, the engine's health and whether a session listens, as the MCP's
observer passes its own: a session attached stands where the MCP reads its
Channel open, both saying whether anyone hears an emergency.
