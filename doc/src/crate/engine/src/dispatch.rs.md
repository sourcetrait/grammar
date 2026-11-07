# dispatch.rs

The engine's two lanes and the calls in flight on them: a call's dispatch, its entry in
the in-flight map, its timeout, and the hang watch a cancelled call leaves behind.

## fn write_definition
PRE-DISPATCH and for EVERY `execute` and `rerun` eval - success, error, or timeout alike. That
is what leaves a timed-out call rerunnable: its error envelope carries the nonce, and
`rerun(nonce, data, timeout: bigger)` recovers it.

Non-fatal by design: a write failure logs and the call proceeds, so the call still returns
its result and nonce and only a later rerun would miss the definition.

## struct InFlightEntry
What the shutdown sweep reads of an eval: its cancel flag, its tracker, and its end.
Nothing lists the table; an await call's row lives through both of its lives, its prepare
within the call, then its job under the same nonce.

## struct HungWatch
Registered on the TIMEOUT branch of either dispatch path, and on an await job's stop.
The timeout path has already triggered cancel and tree-killed the external tree by then, so
this registration is the additional "watch this for a hang" step rather than the response
itself.

`finished` is the deterministic discriminator: the eval thread carries a `FinishGuard` that
flips it on DROP, so a normal return OR a caught panic both flip it, and only a true hang
leaves it false.

## fn register_hung
Idempotent per nonce, because a stop and a later timeout can both fire for one job and the
FIRST stamp is the honest one. Poison-tolerant, like every long-lived lock here.

## fn hung_snapshot
PRUNES AS IT READS. A finished thread is no hang, and the `Health` answer is the registry's
one reader, so the read is where a cancelled call that did end leaves the map. The grace
that confirms a hang is the supervisor's to apply: the snapshot reports every thread still
running, with its cancel time.

## struct Dispatch
One call as both lanes take it, so `execute`, `rerun`, and `call` reach a lane through the
same description: its nonce and source, its tool, the definition to cache, and its
timeout.

## fn dispatch_pooled
A call's own: its log directory made and its definition cached, then `eval_pooled` with its
row in flight. The caller names the log directory, so an await job's prepare logs under
its job's directory as a call logs under its cache kind's.

## fn eval_pooled
THE HOST MINTS THE NONCE and the request carries it; the dispatch hands it to the eval, which
sets it as `$env.NONCE` on the eval's stack - which is what makes the ambient value equal
the envelope's. An eval with no row in flight is an await job's status or stop, which run
under the job's own row; everything else about the pool holds for it.

`refresh_base_if_stale` runs before taking a clone, so this eval runs against current
plugin decls rather than whatever the base held when it was built.

ONE DEADLINE covers the permit's wait and the eval: the call's timeout is what the agent
is willing to wait for the call, so the eval gets what the wait left. Without the bound,
a call arriving while every permit is held - hung threads holding them among the
causes - waited without end, past its own timeout.
A lapsed wait is `thread::dispatch` naming the permits held against the cap, with the
nonce, since the call's log directory already exists.

The tracker is set as the engine's `background_thread_job` BEFORE the eval starts, because
nushell registers an external's pid at spawn time and a tracker attached later would miss
it.

ON TIMEOUT the order matters: trigger cancel so the abandoned thread bails at nushell's
next check point and releases its permit, THEN tree-kill the external process tree, THEN
register for hang watching. A pure-Rust hung eval cannot be reached by the first step -
that is the accepted residual - and the registration is what turns it into a reported
condition rather than a silent leak.

## fn dispatch_stateful
The stateful lane, a `def --env` definition's. The engine handle is cloned OUT of the guard
rather than held across the eval, because the engine thread serializes calls itself -
holding the guard would serialize them twice and block an unrelated caller for the
duration. Its timeout covers the wait for the lane as well as the eval.

## struct InFlightCleanup
Drop SPAWNS an async removal task because `Drop` cannot await, and the map is behind an
async mutex. The consequence worth knowing: removal is not synchronous with the dispatch
returning, so a sweep racing a completion can still meet the finished entry, whose tracker
holds no live pid.

## fn remove_in_flight
The removal is by identity, the row's `finished` flag the eval's own: an await call's
prepare returns and its job registers under the same nonce at once, so a late removal by
nonce alone would take the job's row.

## fn teardown_all_in_flight
The `Teardown` command and the system's stop both run it, so a host shutting down - its
client gone, or a signal taken mid-eval - leaks no process tree. Every await job's stop
runs first, while its job still runs, so the stop can tear down what the job holds.

## fn sweep_env_jobs
THE HALF `in_flight` STRUCTURALLY CANNOT COVER. A `job spawn` that outlives its eval has
had its registry entry removed by `InFlightCleanup` the moment that dispatch returned,
while `env_jobs` still holds it - and its external child is exactly the one that would
otherwise keep running as the host's user with its supervising thread already gone.

The pids are collected BEFORE `kill_all`, for two reasons: that call clears the table and
takes the tracked pid sets with it, and nushell's own kill reaches only the DIRECT child,
so the process table's descendant walk in `tree_kill` is what gets a grandchild.
