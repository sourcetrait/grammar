# remote.rs

The controller's handler of the remote engines' system (`crate/client`'s
`system.rs`): a worker's asks come up as requests it answers, its emergencies
as packets, and the system's status and end are logged as the engine's are.

## struct RemoteRoute
The engine's own route and the controller's emergency policy, the same two the
MCP's engine and operator answer through (`server/controller.rs`), so a remote
eval's sends and a remote operator's emergencies take the same paths a local
one's do.

### fn on_packet
A worker's emergency goes to the policy under its engine's alias, logged and
announced as the MCP's own are, its host named.

### fn on_request
A worker's ask is answered by the route (`server/host.rs`'s `remote_answer`),
on a task of its own as every system's request is.

### fn on_sub
### fn on_closed
Logged as the engine's are (`server/controller.rs`).
