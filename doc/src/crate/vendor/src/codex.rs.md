# codex.rs

## const CODEX_CLIENT
`codex-mcp-client`, the name Codex's MCP client gives itself in `clientInfo`.

## const THREAD_ID_META
Codex names the calling thread in every model-initiated `tools/call`'s `_meta`
as `threadId`, the thread's own identity; its `sessionId` is shared by a root
thread and every fork of it, so the thread id is the one a delivery targets.

## struct DaemonStatus / fn parse_daemon_status
`codex app-server daemon version` answers JSON, `socketPath`, `status`, and
`appServerVersion`; it is read here at the edge into a typed record and nothing
else keeps it as JSON. A blank or missing socket is refused, since the face
connects to nothing else.

## fn daemon_status
The one Codex command run here besides `--version` (User), its stdin closed and
its run bounded, killed past ten seconds: the command probes its daemon within
two, so a run that outlasts the bound is a hang, never a slow answer.
