# host.rs

## fn init_engine
The engine's start: its parameters from the config the process resolved and its
homes the namespace's (`home.rs`). No engine can be built before it, so the
server and the one-shot CLI start it first, after the namespace's lock
(`run.rs`), and the test harness once a process (`guts.rs`).

The scratch directories are made here, before the engine, because parameters
carry them: every definition reads them as `$env.EQUIP_TMP` and
`$env.EQUIP_TRANSIENT` (`scratch.rs`). A directory that cannot be made fails the
start, as any other parameter would. The process's nom rides the parameters the
same way, a definition's `$env.EQUIP_MCP_NOM`.

## fn ensure_substrate
The engine's equip store first, then mcp's purview bookkeeping, the plugin
registry, and the builtins, each where the next needs it. A new namespace gets
the builtins and their purviews; an existing one is never touched, so a builtin
a user uninstalled stays uninstalled, and nothing installs again on their
behalf.

`ensure_default_purview` is non-fatal: purview bookkeeping must not stop the
host from serving.

`ensure_plugin_registry` is non-fatal for the same reason, and runs before the
builtins, since grammar's rig calls plugin commands its validation must know. It
belongs to the substrate because every engine base loads the registry it writes,
so it must land before the first base is built, on every entry point that
starts an engine: the server, the one-shot CLI, and the test server.

The engine starts after it, its locks hydrated from the store this makes, so
they hold the builtins installed a moment before (`run.rs`).

## struct HostRoute
What an eval asks of mcp is answered here: the Channel, the remote links, the
config, and the pins are mcp's. An eval asks its engine's controller, mcp's
(`controller.rs`), and this is the handler the controller hands the engine's
system: it holds the face's two systems, the Channel's and the peer links', and
routes each ask to the one it is for. A subsys system takes only data at its
start, so the engine holds no handle of the face's itself.

### fn on_request
Each ask is answered on a task of its own, as subsys answers every request a
system asks of its controller, so a slow send holds no other ask up. An ask is
always answered, a failure as the route's refusal, so an eval never waits on
one.

### fn on_sub
The engine's status changes are logged, the shutdown's own unsaid
(`controller.rs`'s `log_status`).

### fn answer
A send goes to its system whole; the config is read here, with what each system
holds in force; a pin is the operator crate's live layer, which its watchdog
reads, whether or not an operator runs, since an unsupervised host pins and
reports its pins as a supervised one does.

### fn channel_send
The Channel's system takes the send whole, each step in its order there
(`crate/channel`'s `plan_send`): the reservation, the gate and its unverified
teardown, the spam count, the files and the spill, and the emit. A stop verdict
goes back to the sender, which stops its own eval and reports the action
(`HostAsk::Stopped`), and the report goes on to the Channel's system for the
error's emergency.

### fn remote_send
The send is prepared by the engine's `prepare_send`, its files read and its
event rendered off the runtime's workers, as a worker prepares a remote eval's.
The reservation was refused on the eval's side first, as it always came before
the files.

### fn send_prepared
A prepared send, a local eval's or a worker's, goes to the peer links' system,
which mints the id, spills an oversized event, and queues the frames on the
link to `mcp_nom` (`crate/peer`'s `system.rs`).

### fn remote_answer
A remote engine's ask is answered as a local eval's is, so a definition on a
worker reaches the agent's Channel and the remote links as it does locally. A
Channel send and a spam stop take an origin naming the engine,
`<alias>/thread/<nonce>`, so the Channel counts each remote call apart from
every local one and its packets and emergencies say where they came from. A
prepared send goes to the peer links whole, its files read on the worker. The
config answers from the MCP's, which the worker answers with its own
supervisor lines in place (`crate/daemon`'s `controller.rs`); a pin never
reaches here, the worker holding it.

## fn remote_origin
The worker's `thread/<nonce>` under its engine's alias.

## fn config_record
The record `grimm get_config_all` returns mirrors the config file - the same
tables, the same key names - so one shape describes the file, the `.nutype`
model beside it, and what a definition sees. The remotes file stays out: a
definition addresses a linked host by its nom, which `interface remote channel
list` reports, never by the file's entry.

`identity` is null when the file names none, rather than absent, so the record's
shape holds whatever the file says.

The argument-owned values (`--id`, `--namespace`, `--workdir`) are deliberately
absent: they are not config-file settings, and a definition already reads them
ambiently as the `EQUIP_*` trio.

Values are live rather than as-launched. The channel's spam thresholds come from
the Channel's system (`live_thresholds`), which `interface channel config`
updates, and the supervisor lines come through the pin layer - so what a
definition reads is what is actually in force rather than the startup seed.

It reads through the operator's `effective_supervisor`, the config's lines its
base, rather than the config's lines alone, which is what keeps a pin from being
something the watchdog honors while a read reports the old value.

## fn live_thresholds
A Channel's system that cannot answer leaves the config's start, the thresholds
the system began on.
