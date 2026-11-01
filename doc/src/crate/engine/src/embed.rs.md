# embed.rs

In-process nushell evaluation, the host-side eval engine. The stateless base is built
once and held by the `Executor`, which hands each eval a pre-built clone onto a
dedicated 64 MB blocking thread; the stateful engine is a single long-lived thread.
Cancellation rides a per-eval `Signals` flag the dispatch layer registers in the
resource registry.

## struct FinishGuard
The discriminator between "slow, or bailed on cancel" and "actually hung", and it works
because the drop covers both endings: a normal return and a caught panic both drop the
guard, so both flip the flag. Only a thread stuck where it never returns leaves it
false, which is exactly the condition the watchdog is looking for.

## fn merge_env_no_chdir
Not nushell's own `merge_env`, and this is the single most consequential deviation in
the file. `merge_env` ends by calling `std::env::set_current_dir` with the stack's
`$env.PWD` (nu-protocol 0.116.1, `engine_state.rs`) - correct for a REPL that owns its
process, wrong here. Eval is in-process, so that call would move the whole host's
working directory whenever a stateful definition ran `cd`, and it would stay moved for
the process lifetime, pinning that directory open.

The env-overlay drain mirrors nushell's exactly. The config half goes through the public
`set_config`, which carries the same plugin-GC propagation `merge_env` does inline and
fires it only when the GC config actually changed; the private
`update_plugin_gc_configs` is not reachable, which is why `set_config` rather than a
bare field assignment is the correct substitute.

Dropping the chdir also drops `merge_env`'s only error path, which is why this is
infallible.

`startup` uses it too: nushell's `ensure_path` writes the `PATH` list into the stack
the conversion is given, and nushell keeps that stack for its whole run, so the host
drains it into the engine or keeps a string `PATH`.

Worth knowing even though it does not apply here: nushell resolves an external child's
cwd from `$env.PWD` rather than from the process, so externals behave identically either
way. The difference is visible only to code reading the real process cwd.

## fn eval_in_process
`stdout_file` and `capture_all` compose - they are different layers, not last-wins.
`stdout_file` reassigns the external child's actual fd, while `capture_all` collects the
pipeline value; so an external mid-body writes to the per-nonce log while the terminal
expression's value is what comes back. Verified with N concurrent evals showing zero
cross-talk and nothing on the host's fd 1.

`out_dest` is a per-stack field, which is what makes that concurrency-safe by
construction: applying it to an external `try_clone()`s the File into the child's
`Stdio` per spawn. The only path to the host's real fd 1 is a bare `Stack::new()` with
no redirect at all.

`register_nuapi` must precede the parse, because command names resolve at parse time.
The decls take the eval's host link, its engine's, so a `grimm` ask reaches the
controller of the system that ran the eval; the plugin registration's eval takes none,
its source asking nothing.

`$env.NONCE` goes on the eval's root stack before the parse, rather than into the
source: a custom command's callee stack inherits its caller's env, so the definition's
body, a `def --env` body included, every module it imports, and a `job spawn` it starts
all read the one value, and no template has to splice a line into source the agent
wrote.

The eval answers its nu value, never JSON: `execute` and `call` each check it against
its declared type in its own handler, and the host renders it as NUON.

## fn open_log
Empty, then reopened to append: `print` opens the stderr log itself, by the call's
nonce, and only append keeps its bytes and an external's - which writes through a dup
of this fd - from overwriting each other.

`files_before` is taken before the parse so the cycle check counts only what this parse
registered - the stateful lane's working set accumulates across calls and would
eventually look like a cycle on its own.

The cycle check comes first, before reporting parse errors, because a resolution cycle's
own parse errors are unstable in both variant and payload and useless in every form.

Every nushell error an eval meets, a parse, a compile, a merge, the eval's own, and the
value's collection, reaches the agent through nuval's `NuValError::short`: its message,
its labels, its cause, and its help on one line. A parse error's Debug form put a span
into the engine's whole source in front of an agent, and a shell error's Display alone
dropped the label and help that say what went wrong; the 0.0.0-7 smoke test met both.

`PipelineData::Empty` as the input pairs with `is_mcp = true`, which routes external
stdin to null.

The persist branch strips `$env.NONCE` before merging: it is a per-call ambient, not
session state, so the body's own env writes survive and NONCE does not.

## const EVAL_STACK_SIZE
nu def-recursion is guarded by `$env.config.recursion_limit`, so the only native
overflow left is pathological parser nesting: roughly 1000-deep clears at 8 MB and
roughly 10000-deep at 64 MB. 64 MB clears any realistic body. The residual - deeper
nesting still - aborts the whole host, and that is the locked accepted contract rather
than an open problem.

## fn eval_stateless
The permit rides in the thread. A hung, uncancellable eval therefore keeps its bounded
slot occupied - visible and counted - rather than leaking a thread while freeing the
slot for another hang.

Cancellation rides `cancel`, which becomes this eval's `Signals`, so a kill or a timeout
triggers it and the eval bails at nushell's next check point, dropping its clone and
releasing the permit. A hung thread - a pure-Rust hot loop that never polls Signals -
cannot be reached at all, because you cannot SIGKILL a thread; it runs to its natural
end. External children are reaped separately.

The engine clone drops when the thread returns, which is what reclaims its memory. The
host link rides in the thread beside it.

A caught panic heals the shared jobs table as the stateful thread's does
(`heal_jobs`): the clone's `jobs` is the host's one `env_jobs`, so a panic holding its
lock would leave every later `job` command panicking on the poison. The table is
emptied and its poison cleared.

## fn heal_jobs
Emptying is the only safe heal: a table a panic interrupted mid-update holds nothing
trustworthy. A job thread still running loses its entry and with it its kill handle;
that is the accepted cost, the same the stateful lane's reset pays.

## struct StatefulEngine
The stateful lane, a `def --env` definition's, on the thread `nu-stateful`. The serial
thread is the lane, so a hung stateful eval deadlocks it entirely. That is the case
reset-on-panic does not cover - it handles panics, not a pure-Rust hang - and recovery
there is still an MCP restart.

### fn spawn
Reset-on-panic is broader than it looks. A caught panic may have poisoned the shared
`env_jobs` lock or left the persistent engine inconsistent, so the jobs table is reset
past any poison and the engine rebuilt fresh. Session env, cd and defs are lost; the
host and the lane survive.

The host owning that `Arc<Mutex<Jobs>>` is what makes the poison healable - a field
left buried in an immutable base could not be swapped.

If the thread fails to spawn, which is effectively impossible, `rx` drops with the
closure and every `eval()` surfaces the closed channel as an error rather than hanging.

The tracker is overwritten fresh each eval, so a cancel reaches only the current call's
external children. The host link is taken once, with the thread, the lane being its
engine's for its whole life.
