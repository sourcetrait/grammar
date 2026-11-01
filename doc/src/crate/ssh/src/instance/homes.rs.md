# homes.rs

The worker resolves its homes itself, since it never depends back on the
frontend, in the MCP's order, and its data and cache homes are the MCP's own
roots on every platform, so a worker and an embedded engine find one
namespace's files in one place.

## const DATA, const CACHE
The MCP takes these two roots from `directories`: the UENV variable, else XDG's
on Linux, and on macOS the platform's, where `directories` reads no XDG variable
and answers `Library/Application Support` and `Library/Caches` under `HOME`. The
worker takes them the same way: it serves the MCP's namespace, so it finds the
equips and definitions where the MCP keeps them, and XDG's defaults would part
the two on macOS. `tests/instance_homes.rs` holds the two to `directories`
under one environment.

## struct InstanceHomes
### fn resolve
The environment is data, so a test resolves homes without touching the real
ones, and the caller decides whose environment it is. Each home is its UENV
variable, then the standard one behind it, then its default: `/tmp` for the
temporary home, and the data and cache homes as above. The runtime home has no
default, since a guessed one could sit where another user writes; without one
the transient scratch falls back beside the bulk one (`runtime/scratch.rs`).

### field diagnostics
Each variable ignored for being empty or relative, kept for the caller to
report: a misconfiguration shows rather than quietly moving files.

## fn resolve_home
A chosen base is validated, never made: a home that does not exist is refused
by name, since making a platform's home on a guess could put the worker's files
where the user's tools never look.
