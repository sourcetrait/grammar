# config.rs

`*Toml` is the file layer - every field optional, paths as portable strings,
unknown keys rejected - and `Config` with its sub-items is the format-free
runtime layer. Sub-items get the same pair so a future format adds a shell
without touching the model.

The file carries only what is not an argument. The store namespace (`--id`,
`--namespace`), the work dir and the deny list stay arguments: they identify or
gate the invocation rather than tune it. A file-settable `id` or `namespace`
would let the namespace silently diverge from the `.mcp.json` entry the agent
believes it is talking to, and a sticky default is worse than an explicit
argument. So the two surfaces are disjoint - there is no precedence question
between them at all - and `deny_unknown_fields` turns an attempt to set one from
the file into a loud error rather than a silent no-op.

## const TEST_NAMESPACE
A default, not an override, and the distinction is the whole discipline around
`--test`: the flag names the ordinary test channel without spelling its
namespace out, and says nothing about a run that asks for a different one.

## const MIN_CHANNEL_PORT
Anything below 1024 is privileged and the host is unprivileged by design, so
such a bind could only ever fail. Rejecting it at load turns a confusing
permission error at `interface channel open` into a legible config error at
startup.

## const AGENT_CONFIG_FILE
One file an agent, under the directory the host starts in: an agent's client
starts its host in the agent's own directory. The channel, the supervisor, and
the identity sit together in it, read once at the start, and two agents on one
machine never share one by accident. `--config` names another file in its
place. The remotes live beside it in `remotes.toml` (`remotes.rs`), read at
each use, so they change with no restart.

## struct ConfigToml
The embedded base and the agent's file share this shape. A `[[remote]]` here
is an unknown field, so a file from before the remotes moved fails loudly.

## struct EngineConfigToml
One remote engine, `[[engine]]`, the agent's file's alone: the base names
none. Each field is optional here so a missing one is refused by its name at
resolve, and `port` and `namespace` take their defaults there.

## struct IdentityConfigToml
The host's own key pair, one for everything it presents: the Channel's TLS and
every remote link's mTLS. An agent is one identity, so a pair for each link
would only repeat the same two paths. Both fields are optional here so a
missing one is refused by its name at resolve rather than by serde's generic
message.

## struct SupervisorConfigToml
The operator reads it; a build without one accepts the table and ignores it.

## struct ChannelConfigToml
The port is the only structural key, because everything else about the
channel is fixed by design rather than configured. It is a loopback socket
serving a single host, so there is no address to choose and no peer policy to
express - the bind is 127.0.0.1 by construction, which is what makes "only
localhost gets in" a property of the socket rather than a rule something has to
enforce. What it presents is the host's `[identity]`.

### field port
Modelled as an `Option` rather than a 0 sentinel because 0 is not a port. Zero
is rejected outright rather than treated as "any".

### field spam_warn_window_secs
Windows are integer seconds on this surface for two independent reasons: the
interface's arguments are strings and whole numbers alone, so a nu `duration`
has no shape in a command, and TOML has no duration type either. The `10sec`
form lives in the model and the docs, never in a command or a file.

## struct Config

### field test
A binary flag with exactly two meanings, and deliberately not a mode: it
defaults the namespace to `test` - resolved at the argument boundary, so nothing
downstream re-reads it for that - and it ties the operator's sampling to the
channel's phase. Everything else behaves identically.

### field identity
None when the file names none, which is the embedded base's state: a key pair
is the deployment's own, so there is no default to give. The Channel and each
link refuse to start without it, each at its own open, so a host that opens
neither runs without one.

### field remotes_file
The config file's sibling, whichever file the config came from, so `--config`
moves the remotes with it.

### field engines
The remote engines a tool's `host` names, each checked at load. They are read
once, at the start, as the rest of the file is; the remote engines' system
starts on them (`server/controller.rs`).

## struct ChannelConfig

### field spam
The starting policy, the Channel's own thresholds type (`crate/channel`'s
`spam.rs`). Runtime changes go through `interface channel config`, which
updates the Channel's system rather than this. The config is resolved once and
shared read-only (`run.rs`), so anything adjustable at runtime needs its own
live layer; the operator's pins are the same shape applied to `[supervisor]`.

The starting values are initial. There is no basis for them beyond reasoning; only
production traffic will say what a normal producer actually does. The gap
between legitimate and runaway is enormous rather than marginal - a state lane
emits single digits per burst, a loop emits thousands per second - so the
numbers barely affect detection and mostly decide how often a well-behaved fast
command gets flagged. That is why erring permissive is right here.

## fn resolved_identity
Both files are required and expanded at load, so a missing one or an
unresolvable variable fails at startup with its name, not at the first open.

## fn resolved_engines
Each entry's required fields are present or refused by name, `port` defaults
to 22 and `namespace` to `default`, and the key file expands as every other
path here does. The command and the workdir are the host's paths, not this
one's, so they never expand. Each entry is held to the client's `check`
(`crate/client`'s `endpoint.rs`), and an alias may name one engine alone. An
engine's alias and a remote channel's never meet in one command, so the two
may share a name.

## fn merged_supervisor
The operator reads the table, through `Config.supervisor`.

## fn fraction_field
The range is the operator's `check_fraction`, the one check a runtime pin is
held to as well: a pin must not be able to reach a state a config load would
have refused, and the only way to guarantee that is to share the check rather
than restate it. This adds the file layer's own refusal, a line the merged
layers don't carry.

The callers pass the bare field name, not the dotted key - the check composes
the `supervisor.` prefix itself, so handing it an already-dotted key would
double it to `supervisor.supervisor.cpu_warn_fraction` in the error message. A
test that asserts refusal alone would miss that; the text has to be read back.

## fn secs_field
Zero would mean "no window", which is not a rate at all.

## fn rate_field
Zero would forbid the first send outright rather than police a rate.

## fn from_toml
Deliberately not a `TryFrom`: the model carries fields the format layer does
not, and a named constructor that takes them keeps that asymmetry visible
rather than hiding it behind a conversion.

## fn read_toml
An explicit `--config` that is absent or malformed is an error rather than a
fallback to the defaults, because a typo'd path must fail rather than quietly
serve something else.

## fn read_agent_toml
The agent's own file is optional where `--config` is not: an absent one leaves
the embedded base alone, the ordinary state of a host that opens no Channel. A
present one that does not parse is still an error.

## fn default_id
A one-time process read for the zero-config human case. Harness `.mcp.json`
entries always pass `--id` explicitly, so this default is never what an agent
runs under.

## fn expand_path
Expanding at the boundary is what makes an unresolvable variable a load-time
error naming the variable, rather than a puzzle at first use much later.

The expansion is `shellexpand::full_with_context` behind sourcetrait_common's
guard idiom (see agnostic's `XdgDir::homed`); the dependency's version spec
mirrors common's (`"3"`, no features). The context hits the real environment
first and only then falls back to `var_default`, so a set variable always wins
and the box environment simply overrides the defaults - the same relationship
XDG itself defines. A file naming `$XDG_DATA_HOME` therefore loads on a machine
that leaves it unset, and one naming `$UENV_USR_DATA` on a machine without the
UENV tree. An empty variable reads as unset, as the XDG spec has it for its
own, rather than expanding a path onto the root.

An unknown variable returns `Err`, never `Ok(None)`: shellexpand leaves an
`Ok(None)` reference literally unexpanded in the output, which would pass a
`$VAR` path through as a literal. The function invents no value for a variable
outside the homes.

## fn home_dir
`$HOME` read per call rather than through `BASE_DIRS`: `BaseDirs` snapshots
its environment at first construction, and `expand_path` runs inside
`set_cli_env_path` before the CliEnv overrides land, so touching the LazyLock
here would freeze pre-override state into every later path.

## fn var_default
A UENV home stands in for itself with the standard variable behind it, then
with that variable's default, the order the host reads every home in
(`home.rs`).

## fn expand_defaults
The peer links expand a certificate's path in their own crate (`crate/peer`'s
`remotes.rs`), so `var_default`'s answers cross to them as data, one for each
home variable that has one, read when the face starts
(`server/channel/face.rs`'s `peer_params`). A path there expands as it would
here.

## fn spec_default
The four homes the XDG base directory spec defines, each at its default under
`$HOME`, and `TMPDIR` at `/tmp`, and nothing beyond them. Hardcoded in the mcp
for now: the expansion eventually lives in sourcetrait_agnostic, whose `XdgDir`
keeps its default constants private (ExpandOntoAgnostic).

## enum DeniableTool
The four tools, and an interface command by its words: `--deny 'purview
configure'` withholds that one command while the tool stands, and `--deny
interface` the tool whole. `Command` holds the table's own words
(`server/interface/grammar.rs`), so a deny names exactly a command the parse
registers, and a word sequence the table lacks is refused at the argument as an
unknown tool is.
