# engine.rs

## const NU_VERSION
Read from the lockfile's nu-protocol at build (`build.rs`), so the version a body
sees and the one `interface info` reports are the engine's own.

## fn base_context
THE shared command set, and the sharing is the point: every engine the host
builds starts here, through `startup`, so the eval engines and the equip
validator cannot drift about what command set a rig may legally call. If the
two diverged - the validator missing a layer the eval engine has - a rig calling
`str snake-case` would fail commit while running fine, and the diagnostic would
be an `ExtraPositional` naming the builtin head the parser mis-bound, nothing in
it suggesting a missing engine layer.

`is_mcp = true` is load-bearing and easy to read as decoration. On nu 0.116.1
(`run_external.rs`) it routes an external's empty stdin to null, since the
host's own stdin is the JSON-RPC channel and an external could otherwise steal
the protocol stream; and it runs `setsid` before each external's exec, so every
external leads its own session and process group with no controlling terminal.

`is_interactive = false` keeps nushell's foreground handling out: no external
takes a terminal's foreground group. The teardown still walks the process table
rather than signalling a group, since a descendant that starts its own session or
group leaves the external's (`teardown.rs`).

## fn startup
THE HOST'S ONE STARTUP, following nushell's own `main` (nu 0.116.1,
`src/main.rs`) in its order, so a body sees the shell `nu --mcp -n` gives but
for the host's deltas: the namespace as the lib dirs, the namespace's plugin
registry, the `plugin` commands in the stateful mode alone, the shadowed
host-fatal decls, and a `print` writing to the call's log. Every engine is built
here - the stateless eval base, the stateful engine, the registration engine,
and the parse engine behind the lint and the validator - so validation still
equals serving.

The order carries three constraints:

- `config_dirs` before `generate_nu_constant`: nu-protocol builds every `$nu`
  path from it, and on 0.116.1 an empty `home_dir` makes `$nu.home-dir` raise
  `setting $nu.home-dir failed`, the other paths reading empty.
- `NU_PLUGIN_DIRS`, `$env.config`, and `ENV_CONVERSIONS` before the process
  env, `NU_VERSION` after it: nushell lets a parent's value win for the first
  three and sets its own version last, so the parent shell's `NU_VERSION` never
  survives.
- The shadows after the shell context, since name resolution takes the
  last-registered decl; the standard library and the plugin decls before the
  constant, so `$nu.plugin-path` reads the registry.

A failure converting the env or loading std is reported and survived, as a
failed plugin load is: the engine still serves every body that needs neither.

No `setsid()`: the host is a child of the MCP client and cannot detach its own
controlling terminal. No TLS crypto provider is installed here: mcp names
ring's on each TLS config it builds, the Channel's hub and the remote links,
since nushell reads its own OnceLock once per process.

## fn add_print
On every engine, as nushell's own startup puts `print` on its own, so the
validator parses it as the eval runs it; it finds its call at run time
(`nuapi/print.rs`).

## fn seed_env
The whole OS env goes in because externals need `$env.PATH`.

`NU_LIB_DIRS` is EXCLUDED so the box's own value cannot override the lib dirs
seeded below - the namespace is the sole controlled lib path, and letting the
environment win would let a body reach outside the namespace.

The four EQUIP values are set directly from the engine's params rather than
inherited: eval is in-process with no spawn-env to carry them, and they are not
in the host's own environment, so the engine seeds them from what the host gave
it. They are set after the process env, so a parent's variable of the same name
never stands in for the host's own value.

## fn convert_env
nushell's own conversion: with no `ENV_CONVERSIONS` entries it changes nothing
but `PATH`, which its `ensure_path` turns into a list; an external's child env
gets it back as a string.

## fn seed_lib_dirs
The env var mirrors the constant, as nushell sets both; the constant is what
the parser reads first (`parse_engine.rs`).

## fn seed_version
The build's `NU_VERSION` (`build.rs`, from the lockfile's nu-protocol), the
engine's own.
