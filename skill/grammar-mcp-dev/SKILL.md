---
name: grammar-mcp-dev
description: Authoring, committing, and managing Grammar MCP rigs
---
# Grammar MCP Development
A rig is a nu module tree you own, committed into the MCP's signed repository.
Using the tools is in grammar-mcp; the Nu language is in nu.

## Lifecycle
- `rig(new, rig, source_dir)`: establish an empty rig at a source dir.
- `new([namepaths])`: scaffold modules or functions into an established rig.
- `rig(check, rig, source_dir)`: validate without committing.
- `commit(rig)`: validate and upsert; returns `{added, modified, removed}`.
- `rig(install, rig, source_dir)`: establish and commit a finished source.
- `rig(uninstall, rig, source_dir)`: drop the rig; its source is untouched.

A rig is `<author>/<name>`; a function `<author>/<rig>:module/path:function`.
Every function lives in a module, none at the rig root.

## Call target
A directory `<name>/` whose `mod.nu` exports one `main`:
```nu
# Doubles x.
export def main [args: record<x: int>]: nothing -> record<y: int> {
    { y: ($args.x * 2) }
}
```
- The `args` record type is the args schema; the output type the result's.
- Either may be `nothing` for void; a bare `record<>` is rejected.
- Its `mod.nu` may hold private helpers, never a submodule.
- `main` is reserved for this export alone.

## Wiring
Each directory's `mod.nu` wires its children:
- call target: `export module <name>` and `export use <name>`
- pure module, no `main`: `export module <name>` only
- flat helper file: `export use ./<name>.nu` or a private `use`; no `main`
```
geo/
  mod.nu             # export module shape
  shape/mod.nu       # export module area; export use area
  shape/area/mod.nu  # export def main ...
```
Wiring errors: `rig::call_wiring` (no `export use`), `rig::orphan` (unwired),
`rig::call_leaf` (a call holding a submodule).

## Imports
Always the full path, your own rig included, then drive by module name:
```nu
# text/slug/mod.nu
use rig/acme/toolkit/text/common
export def main [args: record<s: string>]: nothing -> record<out: string> {
    { out: (common normalize $args.s) }
}
```
- Import a sibling module directly, never your parent (`CircularImport`).
- Import a single function only to resolve a name collision.
- `./x.nu` is only for flat helper files.
- Another rig must be committed before yours can import it.

## Docs
The comment above `main` documents a call; a `mod.nu`'s leading comment, its
module, or the rig at the root.
Its first line is the summary (≤80 chars); details follow a blank comment line.
Say only what names and types don't: units, side effects, edges, failures.

## Files
A commit carries only:
- `.nu` sources and `.gitignore` files
- root `rig.toml`, `README.md`, `LEGAL.md`, `LICENSE.txt`, `LICENSE-*.txt`
- root `.assets/`: data, no script extensions
- root `.docs/`: `.md` and `.txt`

Nothing may be executable; anything else is a `rig::*_denied` error.
A rig can't be named `docs`, `tools`, `bin`, or `target`.

## Where rigs live
- repository-local: `./nu/<name>/`
- shared: symlink `$XDGX_LIBRARY_HOME/nu/rig/<author>/<name>` to the source
