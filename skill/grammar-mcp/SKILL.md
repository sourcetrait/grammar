---
name: grammar-mcp
description: Grammar MCP tools - run, call, schemas, lint, shm, channels
---
# Grammar MCP
Nu bodies evaluated by the MCP. The Nu language itself is in the nu skill.

## Tools
- `run()`: evaluate a typed nu body on a stateless thread.
- `interact()`: the same on a persistent thread; env, `cd`, and defs carry over.
- `call()`: invoke a committed rig function by namepath.
- `rerun()`: re-evaluate a cached `run()` body by its nonce with fresh args.
- `processes()` / `kill()`: list in-flight calls; cancel one by nonce.
- `info()`: versions, plugins, and the rig signatures in view.
- `inspect()`: docs and source for a namepath or a pattern.
- `purview()` and kin: scope what `info()` shows (Purviews below).
- `channel_*` / `remote_channel*`: notification lanes (below).

Rig authoring (`rig`, `new`, `commit`) is in GrammarMcpDevelopment.

## run()
`args` binds to `$args`; the body's last value must match `result_schema`.
```json
{"args_schema": {"root": "string"}, "result_schema": {"n": "int"},
 "args": {"root": "/tmp/demo"},
 "body": "{ n: (glob ($args.root | path join '*') | length) }"}
```
Returns `{result, nonce}`, or `{error: {errors, warnings, nonce?}}`.

## Schemas
Field name to type; records and tables are open, every declared field required.
- scalars: `int float string bool number datetime duration filesize binary`,
  `range glob cell-path path directory`
- record `{"x": "int"}`, list `["string"]`, table `[{"name": "string"}]`
- union `{"oneof<>": ["int", null]}`; `null` is nothing, valid only in a union
- `{}` as the whole schema is void; a nested `{}`, `[]`, or `any` is refused
- `[]` satisfies any list or table type

Args are deep-checked at runtime; a result only when its shape is static.

## Lint
A run or interact body is rejected before evaluation for:
- `lint::hardcoded_variable`: any literal starting `/` `~/` `./` `../`; use args
- `lint::denied_command`: one of these run with `^`; use a builtin instead:
  `awk bash cp date echo find grep jq ls nu rg rm sed sh sort which`

`source`/`use` paths must be constants: inline the code or call a rig.

## Errors
- `lint::*`: the body broke a lint rule and never ran.
- `thread::returned_error`: parse, type, external exit, or unknown command.
- `thread::timeout`: past `timeout_ms` (120000), or an external awaiting stdin.
- `thread::dispatch`: the eval pool was busy; not fixable from the body.

Each carries a nonce, which keys the body's stdout/stderr in the cache log dir.
"External command failed" with empty logs usually means a misspelled command
or a line-leading `else`.

## Ambient env
- `$env.NONCE`: this call's nonce, seen by the body and everything it calls.
- `$env.EQUIP_{ID,NAMESPACE,WORK_DIR}`: the server's id, namespace, work dir.

## Large payloads: shm
Prefer a summary; move bulk too big for a result through `$env.XDGX_SHM_DIR`.
Pass paths relative to it in fields named `shm_*` or `*_shm`.
A body writes in `mcp/run/<nonce>`, `mcp/interact/<nonce>`, or for a call
`<author>/<rig>/<module>/<nonce>`; you write wherever you choose.
```nu
let rel = ('mcp/run' | path join $env.NONCE 'data.json')
mkdir ($env.XDGX_SHM_DIR | path join ($rel | path dirname))
$data | to json | save -f ($env.XDGX_SHM_DIR | path join $rel)
{ shm_out: $rel }
```
shm isn't for logs; the cache log dir already holds a body's output.

## Rigs in a body
Import the module, never the rig root, then drive it by name:
```nu
use rig/<author>/<rig>/<module>
<module> <export> <params>
```

## grimm (body only)
- `grimm dbg <data>`: append a record or table to this call's `debug.nuonl`.
- `grimm channel send <model> <event> [attached]`: emit on your own channel.
- `grimm remote channel send <peer_nom> <model> <event>`: to a linked host.
- `grimm remote_channel_send_with <peer_nom> <model> <event> <attached>`:
  the same, with `attached` a `table<src, dest>` of files.
- `grimm get_config_all` / `get_config <key>`: read the live config.
- `grimm pin_config <key> <pid> <value>`: pin `supervisor.*` until pid exits.

## info() and inspect()
`info().signatures` indents one space a level: author, rig, modules, calls.
A call line ends in `<args> <result>`, `<>` when void.
```txt
acme
 geo # planar geometry helpers
  shape
   area <width:float,height:float> <area:float> # inputs' unit squared
```
`area` is `call("acme/geo:shape:area", {width: 3.0, height: 4.0})`.
`inspect()` takes an exact namepath or a pattern: `author/`, `author/rig:`,
`.../mod/` its subtree, `.../mod:` its calls, `*` everything, `.` the purview.

## Purviews
Named namepath patterns that filter what `info()` shows; not access control.
- `default`: a fresh session's view, `['*']` unless configured.
- `.`: the current view; `*`: everything; `@id`: another purview's patterns.
- `purview(ids)` replaces the view; `purview_extend(ids)` adds to it.
- `purview_configure(id, patterns)`: set one; `[]` deletes, or resets `default`.

`info(purviews: [...])` renders another view without switching.

## Channel
A loopback WSS lane for a body to notify you: small summaries, never payloads.
1. `channel_open()` → `{status, wss, inbox}`; `existing` is normal, keep it.
2. Start a persistent Monitor on `wss`.
3. Wait for the `mcp/channel/Open` packet.
4. `channel_verified()`; sends before it are refused.
5. `channel_close()` when done.

A packet is `{id, from, model, event, attached?}`; read it with `from nuon`.
Models under `mcp/` are reserved for the host.
An oversized event spills to the file `<inbox>/<spilled_event_path>`.
Sending too fast warns, then stops the sender; `config_channel()` tunes it.
On `channel::peer_gone`, close and reopen.

## RemoteChannel
Links two hosts' channels by an alias in `.grammar/mcp/remotes.toml`.
Peers trust each other by known public key.
1. Both agents bring their channel up and verified.
2. `remote_channel_open(alias)`: the listener first, then the connector.
3. Send to the peer's nom from `remote_channels()`, never the alias.
4. Expect `mcp/remote/Sent` or `Unsent`; replies come from `mcp/remote/<nom>`.
5. `remote_channel_close(alias)`.
