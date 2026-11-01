# template.rs

The two source builders, `execute`'s and `call`'s. Each embeds a NUON literal the host
rendered rather than leaving the eval to re-parse JSON, so the parser downstream only
ever sees well-formed nu. Neither sets `$env.NONCE`: `eval_in_process` puts it on the
eval's stack, where every command the source reaches finds it.

## const CALL_ALIAS
A constant we own, and that ownership is the whole point. A call target's name is the
agent's to choose and may collide with a nushell keyword, which can never be shadowed -
the keyword wins at parse under `use`, `overlay use` and `overlay use --prefix` alike.
`run` became a keyword in 0.114 and there is a `run` call target in the estate, so
aliasing the overlay sidesteps the namespace entirely and the drive is a constant
rather than the target's own name.

## fn build_call_source
Four pieces, each load-bearing and none of them obvious:

The overlaid path is absolute, the target under the namespace's equip root, as a
string literal. nushell resolves a relative module path against the parsing file's
directory, or `$env.PWD` when there is none, before `$NU_LIB_DIRS` (nu-parser
0.116.1, `find_in_dirs_with_id`). A call's source is no file, so a relative
`equip/...` would resolve against the host's start directory first, and an agent
starting its host in its own repository, its equips' sources at that same path,
would run its uncommitted source in place of the commit.

The overlaid path is the target's own module under the equip's rig,
`<author>/<name>/rig/...`, not the rig root. 0.114 (PR #18303) stopped implicitly
importing a module's submodules, so the pre-0.114 form - a root `use` plus a
module-qualified drive - binds nothing the drive can traverse and is dead.

`as __call` sidesteps the keyword collision described under `CALL_ALIAS`.

`--prefix` matters because a call target may export helpers beside `main`. Without it
the overlay flattens them bare into the eval scope, where one named `open` would shadow
the builtin for everything else in that eval.

Importing the target alone rather than the whole rig costs nothing: the target's own
`use equip/...` references bind at that file's parse, against its own directory in the
namespace and then the const `$NU_LIB_DIRS`, never through this import, so a start
directory cannot reach them.

The data's literal is the engine's render of the call's data once it holds to `main`'s
type, as an `execute` source's is; the timeout's follows only when `main` takes
`timeout?`.

## fn build_execute_source
The definition as the agent wrote it, a newline, then the one call. The newline is
load-bearing: a definition ending in a comment would otherwise swallow the call. The
contract has already held the definition to exactly one top-level `def execute`, so the
source holds nothing else that runs.

No wrapper scopes it. On the stateless pool the clone drops with its thread; on the
stateful lane `def --env` is what carries the definition's `$env` and `cd` to the
eval's top, where the engine's merge persists them, the def's own `let`s and `def`s
staying inside it. The decl `execute` itself persists in the stateful engine, each
call's later def winning its own call's resolution.

The data's literal binds the `data` positional, so nushell's own check of it runs at
parse too, beneath the host's strict one; the timeout's literal follows only when the
signature takes `timeout?`.
