# remote_channels.rs

## fn remote_answer
Each remote command is the peer links' system's (`crate/peer`), its answer the
wire's result or refusal, rendered here as the tool always rendered it: the
rows, a row, the endpoint, or nothing. The refusals keep their kinds,
`remote::channel::*` and `remote::file`, through the runtime error the face
converts.

## fn remote_gate
A listen or a connect needs a verified Channel, since a link's reports and
relays ride it. The face is the controller of both systems, so it reads the
Channel's state before routing either; a close needs none.
