# homes.rs

The worker resolves its homes itself, since it never depends back on the
frontend, in the MCP's order, and its data and cache homes are the MCP's own
roots on every platform, so a worker and an embedded engine find one
namespace's files in one place.

## const DATA, const CACHE
Each is its UENV variable, then XDG's, then the platform's folder, the order the
MCP reads its two roots in on every platform. The folder is what `directories`
answers the MCP: XDG's default under `HOME` on Linux; `Library/Application
Support` and `Library/Caches` under `HOME` on macOS; and on Windows the roaming
and the local application data folders, which the worker reads as their
variables, `APPDATA` and `LOCALAPPDATA`, each a session's own. `directories`
reads XDG's variables on Linux alone, so the worker and the MCP each read them
first themselves. The worker serves the MCP's namespace, so it finds the equips
and definitions where the MCP keeps them. `tests/instance_homes.rs` holds the
worker to the MCP's order under one environment, XDG's variables set and unset.

## const TMP
On Windows the temporary home is `TMP`, then `TEMP`, the order Windows' own
`GetTempPath2W` reads them in, where Unix's is `TMPDIR`, then `/tmp`.

## enum Default
`UnderHome` and `Absolute` serve Unix's defaults, `Var` Windows' variables read
last, `TEMP` and the two application data folders; each platform keeps only the
kinds it uses.

## struct InstanceHomes
### fn resolve
The environment is data, so a test resolves homes without touching the real
ones, and the caller decides whose environment it is. Each home is its UENV
variable, then the standard one behind it, then its default: `/tmp` or `TEMP`
for the temporary home, and the data and cache homes as above. The runtime home
has no default, since a guessed one could sit where another user writes;
without one the transient scratch falls back beside the bulk one
(`runtime/scratch.rs`).

### field diagnostics
Each variable ignored for being empty or relative, kept for the caller to
report: a misconfiguration shows rather than quietly moving files.

## fn resolve_home
A chosen base is validated, never made: a home that does not exist is refused
by name, since making a platform's home on a guess could put the worker's files
where the user's tools never look.
