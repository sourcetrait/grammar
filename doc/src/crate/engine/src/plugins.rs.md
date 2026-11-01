# plugins.rs

One module serves both the eval engines' plugin LOADING and the host's plugin
ENUMERATION, deliberately, so the two views of the registry cannot drift.

THE MCP OWNS ITS REGISTRY. Its plugins are the ones it ships, built at its own
engine's version, never the user's shell registry: nu-plugin-protocol loads a
plugin only where the two versions caret-match, so a system registry kept for
another nushell is a registry of plugins that fail to load here.


## fn registry_path
Per NAMESPACE, in its data dir beside the equip repository: a plugin added on the
stateful lane is that namespace's state and persists with it. Two binaries serving
one namespace take turns: nushell's upsert matches by plugin NAME, so each
registration replaces the other's entry rather than doubling it.

None until `init` gives the engine its params, never a panic: an engine built
where no host started one simply carries no plugins.

## fn read_registry
Silent-skip is the discipline here, not laziness: a missing or unreadable
registry has to leave every non-plugin body fully usable, so a failure at any
step returns None and the engine simply carries no plugin decls.

## fn registry_mtime
MTIME RATHER THAN CONTENT, because it is the one signal that catches every way
the registry can move: the startup registration, a `plugin add/rm` on the
stateful lane, and an edit from outside.

The Executor snapshots it when it builds the stateless base and re-stats it once
per dispatch. That is affordable only because a stat is roughly
sub-microsecond - the check is on the hot path, and it fires a rebuild only on an
actual change.

## fn plugin_dir
A sibling resolved from the executable, the environment first. The default is
the OptZst layout, `bin/grammar-mcp` beside `plugin/`; a cargo target dir has no
`plugin/`, so a dev or test host carries no plugins unless the override names a
directory. No layout is guessed between the two.

## const PLUGIN_KINDS
The plugin directory keeps its plugins apart by kind: `core/` nushell's own,
pulled from crates.io at the engine's version, `grammar/` the grammar's, and
`rig/` custom ones. The kinds are read alone, so a binary loose in the plugin
directory, or under any other name, is never registered; a kind not present
is simply empty.

## fn bundled_plugins
Canonicalized, because the registry records the path `plugin add` resolved, and
the staleness test compares against it. dunce canonicalizes, so on Windows the
registry holds the plain path, never std's verbatim `\\?\` form, which the
plugin's own process and a reader of the registry would both meet.

## fn ensure_plugin_registry
Registration spawns each plugin to read its signatures, so it runs only for a
binary the registry lacks or one whose file changed after the registry was
written - a new install, a rebuilt plugin, or another binary's turn at the
namespace.

## fn changed_time
CHANGE TIME, NOT MODIFICATION TIME. An OptZst unpacks every file at mtime 0,
its archive reproducible, so a plugin's mtime predates any registry and a
plugin replaced in place kept its old signatures: measured, a host restarted
over a replaced plugin dated at the epoch left the registry unwritten.

On Unix it is the inode's ctime. The kernel sets ctime whenever a file is
written, copied, unpacked, or renamed into place, and no archive can carry
one, so a changed plugin always counts as newer than the registry, at the cost
of one more registration after a mere `chmod`.

On Windows it is `FILE_BASIC_INFO.ChangeTime`, read from the opened file by
`GetFileInformationByHandleEx`: stable std reads no change time there, its
`MetadataExt::change_time` nightly-only. It is the NTFS change time, which a
write and a metadata change both move, as ctime does, so an attribute change
costs the same one more registration a `chmod` does.

Where the change time can't be read, on either platform, the plugin counts as
changed and registers again.

## fn register_plugins
NUSHELL'S OWN `plugin add`, evaluated on a stateful base, rather than a second
registration path through nu-plugin-engine: what lands in the file is exactly
what the user's `plugin add` would write. Its output goes to the cache dir like
any eval's, and a failure is reported and survived, since a host without plugins
still serves every other body.

## fn load_plugin_decls
Registers DECLS ONLY - no plugin process spawns here, because nushell spawns
those lazily on first invocation. That is what makes the same call cheap enough
for the equip validator to make too: it pays the registry read and nothing else.

`plugin_path` is set here, where the registry is read, as nushell sets it only
when it reads one; the config dirs' `plugin_file` names the same file.

The failed-plugin count gets one summary line to stderr rather than a per-plugin
report, since `load_plugin_file` has already printed each individual error by the
time it returns.

## fn list_registered_plugins
`version` is present only when the plugin author chained `.with_version` when
building their plugin, so a `null` here says nothing about whether the plugin
works. The versions this reports are each plugin's own self-reported crate
version and are NOT an engine-compatibility signal - a plugin reporting an older
number can be perfectly current against the pinned engine.
