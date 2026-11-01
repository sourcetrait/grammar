# config_dirs.rs

## static CONFIG_DIRS
Resolved ONCE, as nushell resolves them at startup: nu-config's contract is that
nothing re-reads the `XDG_*` vars afterwards, so every engine the host builds
takes a clone of these.

## fn resolve_config_dirs
nushell's own `resolve_paths`, never a hand-rolled path: `$nu.default-config-dir`
and the rest name what `nu -n` names for the same env, though the host loads
none of their files. The one override is the plugin file, the namespace's
registry, which `plugin_path` names too, so `$nu.plugin-path` reads the same
either way. The env is a parameter so a test can pass nu-config's `TestEnv`.

## fn init_config_dirs
From the engine's `init`, which the host calls once its CLI env and its config
are set: the CLI may move the XDG vars, and the registry's path needs the
namespace's data home. No config dir is a startup failure, which the host
exits 2 on as for a bad config, since every `$nu` path would be wrong; a
warning goes to stderr, as nushell prints it.
