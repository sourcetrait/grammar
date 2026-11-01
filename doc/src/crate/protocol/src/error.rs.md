# error.rs

The runtime's and the transport's errors, apart from the engine's own. Each
renders to the frontend's envelope as rows of `Diagnostic`s, the same table every
failure has always carried, so a refusal reads alike whether this host or a worker
raised it.

## const LOCAL_HOST
`local` is hard-coded and every other alias is the configuration's, User's ruling,
so this host's alias can never be configured away.

## enum RuntimeError
What the runtime refuses outside the engine: an interface call's violations, an
unknown host, a schema, a purview, the Channel, and the remote Channel. A
purview's answer carries it across a session whole.

### fn kind
The violations variant `unreachable!()`s here deliberately: it renders through
`bucket`, and reaching this arm would mean a caller bypassed that path.

The remote kinds name their remote's kind, `remote::channel::`, as the commands
do, so a remote engine's sit apart under `remote::engine::` (User).

`remote::channel::open_failed` is the synchronous bind or connect failure of a
blocking `interface remote channel listen` or `connect`: distinct from the async
`remote/channel/Disconnected {error}` Channel model, which fires only for a
listener's post-bind accept failure or an established link's later teardown.

`remote::file` is the remotes file's own refusal, a file that doesn't parse or
holds a malformed entry, named with its path; a bad value a command was given is
that command's `remote::channel::invalid_params` instead.

An `interface` call's refusal is `interface::invalid` rows, gathered under
`InterfaceViolations`, each at its position in the command; a command that parsed
and ran refuses with its own kinds, as its tool did.

`remote::channel::unverified` refuses an `interface remote channel listen` or
`connect` while the Channel isn't verified, before any bind or connect: the link's
notices and relays ride the Channel's gate and would drop.

`channel::wrong_mode` names the mode the Channel is open in and the one asked for:
an open in the other mode, or a sync on an async Channel, each told to close
first. `channel::not_claimed`'s message follows the mode, a Monitor to start or a
sync to call.

`host::unknown` is a tool's `host` alias naming no host this MCP knows, its message
naming `local`. The engine's own `host::unserved` stays apart: that is a request
naming a host its engine doesn't serve, which no tool sends.

## enum TransportError
The path to a worker failing, never its engine's answer, each under a stable
`daemon::` kind. The kinds split on whether anything ran. `ConnectFailed`,
`HostKeyRejected`, and `AuthenticationFailed` fail before any request is sent,
`Busy` refuses one before the engine takes it, and a hello's mismatches before
the session serves anything, so each is safe to retry. `SessionLost` is the
one that may have run: the session ended with the request in flight, and a
result is never kept past its session, so its outcome is unknown and nothing
resubmits it; an agent tells a retry that is safe from one that may repeat an
effect. `Unavailable` is a worker that couldn't start, its own failure's kind
and text in the reason, a held namespace among them. `ResultTooLarge` is an
answer past the session's frame or its decode budget, refused alone while the
session goes on. Each new variant goes last, so the ones before it keep their
wire indices.
