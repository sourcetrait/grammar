---
name: grammar-mcp-dev
description: Authoring, installing, and managing Grammar MCP equips and rigs
---
# Grammar MCP Development
An equip is a nu module tree, `equip/<author>/<name>/`: `lib/` its plain
modules and `rig/` its calls, installed together; a `cli/` stays behind.
Using the tools is in grammar-mcp; the Nu language is in nu.

## Lifecycle
- A source is the `equip/<author>/<name>` directory; `commit` takes the
  equip's namepath, `<author>/<name>`.
- A call runs the committed copy: edit the source, then commit.
- `install` adds the rig to `default` and the purviews in view; `uninstall`
  drops it from every purview and answers `{}`.
- A builtin, `gram` or `grammar`, drops its author everywhere:
  `interface equip commit gram`.

## A call
A rig directory `<module>/<call>/mod.nu` exporting one `main`, held to
`execute`'s contract, `data` and an optional `timeout`, without its lint:
```nu
# Doubles x.
export def main [data: record<x: int>]: nothing -> record<y: int> {
    { y: ($data.x * 2) }
}
```
- `main` names a call's export alone: never a module, file, variable, or
  parameter, nor an export of a flat file.
- A call's `mod.nu` may hold private helpers, never a submodule.
- Every call sits in a module of the rig, none at its root.

## Wiring
Each `mod.nu` in the rig wires its children:
- a call: `export module <name>` and `export use <name>`
- a module with no `main`: `export module <name>`
- a flat helper file: `export use ./<name>.nu`, or a private `use`
```txt
rig/
  mod.nu             # export module shape
  shape/mod.nu       # export module area; export use area
  shape/area/mod.nu  # export def main ...
```
`rig::call_wiring` is a call missing its `export use`, `rig::orphan` an
unwired module, `rig::call_leaf` a call holding a submodule.

## Imports
Always the full path, your own equip's included, then drive by module name:
`use equip/<author>/<name>/lib/<module>`, or `.../rig/<module>` a rig's.
- Import a sibling module directly, never your parent
  (`module::circular_import`).
- Another equip is installed before yours can import it.

## Docs
The comment above `main` documents a call; a `mod.nu`'s leading comment, its
module, or the rig at the root. Its first paragraph is the summary, 80
characters at most or the commit warns; the rest is `interface help`'s
details. Say only what names and types don't: units, side effects, edges,
failures.

## Files
A rig carries `.nu` sources and `.gitignore` files; at its root `rig.toml`,
`README.md`, `LEGAL.md`, `LICENSE.txt`, `LICENSE-*.txt`, `.assets/` with no
script, and `.docs/` of `.md` and `.txt`. A lib carries `.nu` files alone.
Nothing is executable. An equip can't be named `docs`, `tools`, `bin`, or
`target`.

## Where equips live
- the repository's own: `./equip/<author>/<name>/`
- shared: linked into nushell's library path,
  `$nu.default-config-dir/scripts/equip/<author>/<name>`
