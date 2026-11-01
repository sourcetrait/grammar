---
name: grammar-mcp
description: Harness shell tools powered by Nushell
---
# Grammar MCP
What the tool and server schemas leave out. The Nu language is in the nu
skill; equip authoring is in grammar-mcp-dev.

## In a definition
- `$env.NONCE`: this call's nonce. `$env.EQUIP_ID`, `EQUIP_NAMESPACE`,
  `EQUIP_WORK_DIR`, and `EQUIP_MCP_NOM`: the host's. `$env.EQUIP_TMP`: its bulk
  scratch directory; `$env.EQUIP_TRANSIENT`: its directory for small runtime
  files.
- Import a lib's or a rig's module by its path, never an equip's root:
  `use equip/<author>/<name>/lib/<module>`, then `<module> <export> ...`.
- A check reports at most three errors at a time; fix them and resubmit.
- `thread::dispatch`: the eval pool was busy; retry.

## Bulk
A result carries a summary. Bulk goes to a file under `$env.EQUIP_TMP`, its
path relative to it in a field named `*_tmp`. The directory is the host's
own, gone when the host ends; `interface info`'s `tmp` names it.

## grimm
The commands a definition reaches the host with:
- `grimm dbg <record|table>`: append to this call's `debug.nuonl`.
- `grimm channel send <model> <event> [attached]`: a packet on your Channel,
  `attached` kept in its inbox; answers `{event_id}`.
- `grimm remote channel send <nom> <model> <event>`: the same to a linked
  host; `grimm remote_channel_send_with` adds a `table<src, dest>` of files.
- `grimm get_config <key>` and `grimm get_config_all`: the config in force.
- `grimm pin_config <key> <pid> <value>`: hold a `supervisor.*` line while
  the pid lives.

## info and help
`interface info` indents one space a level: author, rig, modules, calls. A
call's line is `<name> <data> <result> # <summary>`:
```txt
acme
 geo # planar geometry helpers
  shape
   area <width:float,height:float> <area:float> # the inputs' unit squared
```
That is `call` on `acme/geo:shape:area` with `{width: 3.0, height: 4.0}`. A
builtin, `gram` or `grammar`, sits at an author's depth and is called by its
bare name: `gram:get:log`. `interface help` takes a namepath or a pattern:
`author/`, `author/rig:`, `.../mod/` its subtree, `.../mod:` its calls, `*`
everything, `.` the purview. `interface help interface` gives every
`interface` command's signature, the block that ends this skill, and
`interface help grimm` every `grimm` command's.

## Purviews
Named namepath patterns filtering what `info` shows, never access control:
`default` a fresh session's view, `core` the builtins, `.` the current view,
`*` everything, `@id` another purview's patterns.

## Channel
Packets from a definition to you: small summaries, never payloads.
- async: OpenGrammarChannel.
- sync: `interface channel open sync`; the first `interface channel sync`
  takes the greeting, then `interface channel ready`; each sync after takes
  what queued since.
- A send before `ready` is refused and closes the Channel.
- A packet is `{id, from, model, event, attached?}`; models under `mcp/` and
  `remote/` are the host's. An oversized event spills to
  `<inbox>/<spilled_event_path>`.
- Sending too fast warns, then stops the sender.

## RemoteChannel
Links two hosts' Channels. `.grammar/config.toml`, in the directory the host
starts in, names its key pair as `[identity]`; `.grammar/remotes.toml` beside
it names each peer as a `[[channel]]` alias with its `cert`, read at each
command. Peers trust each other by known public key. A send reports
`remote/channel/Sent` or `Unsent`; a reply comes from `remote/channel/<nom>`.

## SEQUENCE: OpenGrammarChannel
1. `interface channel open async`.
2. On `new`, a Monitor on the returned `wss`; on `existing`, the running
   Monitor; on `channel::peer_gone`, `interface channel close`, then step 1.
3. Wait for the `mcp/channel/Open` packet.
4. `interface channel ready`, nothing sent before it.

## SEQUENCE: GrammarRemoteChannel
1. OpenGrammarChannel on both hosts.
2. Each host without the peer's entry:
   `interface remote channel set <alias> --cert <peer's cert>`.
3. The listener: `interface remote channel listen <alias>`; its `endpoint` to
   the connector.
4. The connector: `interface remote channel connect <alias> --address
   <endpoint>`.
5. The listener waits for `remote/channel/Connected`.
6. Each sends to the peer's nom from `interface remote channel list`, never the
   alias.

## Command Signatures
Note: Each begins with `interface `
### General
MCP details, Equipment signatures within current purview
`info []: nothing -> record`
Displays equipment documentation
`help [namepath: string]: nothing -> record`
### Process
List in-flight MCP tool usage.
`process list []: nothing -> record`
Cancel an in-flight usage by its nonce.
`process kill [nonce: string]: nothing -> nothing`
### Purview
List every configured purview, and what is currently in view.
`purview list []: nothing -> record`
Set the current view to these purviews.
`purview set [purviews: list<string>]: nothing -> record`
Bring more purviews into the current view.
`purview extend [purviews: list<string>]: nothing -> record`
Set a purview's namepath patterns; an empty list deletes it.
`purview configure [purview: string, namepaths: list<string>]: nothing -> record`
### Channel
Opens the Channel
`channel open [mode: string]: nothing -> record`
Takes the packets a sync Channel queued since the last sync.
`channel sync []: nothing -> record`
Verifies Channel activation and begins normal use.
`channel ready []: nothing -> nothing`
Closes the Channel
`channel close []: nothing -> nothing`
Read or adjust the channel's send-rate thresholds at runtime.
`channel config [--spam-warn-window-secs: int, --spam-warn-rate: int, --spam-error-window-secs: int, --spam-error-rate: int]: nothing -> record`
### Remote
List the remote channels, configured and live: each one's entry and state.
`remote channel list []: nothing -> table`
One remote channel's entry and state.
`remote channel get [alias: string]: nothing -> record`
Make or change a remote channel's entry in `remotes.toml`.
`remote channel set [alias: string, --address: string, --listen: string, --cert: string]: nothing -> record`
Remove a remote channel's entry from `remotes.toml`; a live link stays.
`remote channel unset [alias: string]: nothing -> nothing`
Listen for a remote channel's peer; blocks on the bind, answers its endpoint.
`remote channel listen [alias: string, --on: string]: nothing -> record`
Connect to a remote channel's peer; blocks on the connect.
`remote channel connect [alias: string, --address: string]: nothing -> nothing`
Close an open mTLS link to a remote grammar host.
`remote channel close [alias: string]: nothing -> nothing`
List the remote engines, configured and live: each one's entry and state.
`remote engine list []: nothing -> table`
One remote engine's entry and state.
`remote engine get [alias: string]: nothing -> record`
Make or change a remote engine's entry in `remotes.toml`.
`remote engine set [alias: string, --host: string, --port: int, --user: string, --key: string, --host-keys: list<string>, --command: string, --id: string, --namespace: string, --workdir: string]: nothing -> record`
Remove a remote engine's entry from `remotes.toml`; a live session stays.
`remote engine unset [alias: string]: nothing -> nothing`
Open a remote engine's session and start its worker now.
`remote engine connect [alias: string]: nothing -> record`
Close a remote engine's session; its worker ends with it.
`remote engine close [alias: string]: nothing -> nothing`
Make or change a remote engine's entry from an ssh configuration's host.
`remote engine import [alias: string, --from: string, --ssh-config: string]: nothing -> record`
### Equip
Register a new equip at its source and seed its rig.
`equip new [source: string]: nothing -> record`
Validate an equip's whole source and install it.
`equip install [source: string]: nothing -> record`
Validate an installed equip's source, writing nothing.
`equip check [source: string]: nothing -> record`
Remove an installed equip from the namespace; its source stays.
`equip uninstall [source: string]: nothing -> record`
Scaffold modules / functions (by namepath) into an equip's rig.
`equip scaffold [namepaths: list<string>]: nothing -> record`
Commit an equip's lib and rig from its source to the MCP's repository.
`equip commit [namepath: string]: nothing -> record`
