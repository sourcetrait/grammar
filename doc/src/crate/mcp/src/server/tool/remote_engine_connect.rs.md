# remote_engine_connect.rs

`interface remote engine connect` and `close`: a session opened before any
call needs it, and ended before the MCP is.

## fn NuSh::engine_connect
The entry must be whole. Its session opens now, the worker starting with it,
so an agent learns the host is reachable and trusted before it sends work. A
session already open answers as it stands, on the entry it opened on: a change
takes effect at the next session. A failure answers under its transport kind
and stays in the row's `failure` until an open succeeds.

## fn NuSh::engine_close
The session and its worker end, as the MCP's own end ends them, a call in
flight answering `remote::engine::session_lost`; the alias's next call opens a
new session. With no session open the close answers
`remote::engine::not_open`.
