# executor.rs

## fn eval_concurrency_cap
Halved rather than the full parallelism because eval runs on a dedicated blocking
thread and the host itself needs cores for the async runtime and the supervisor. Eval is
in-process, so there is no separate eval or reaper process to reserve cores for.

## struct Executor
The ready buffer holds pre-built clones, not reused ones. Each clone is SINGLE-USE
and drops after its one eval, which is what keeps statelessness intact. `take_clone`
pops a ready clone and then refills the buffer before it returns, both on the
dispatch path, so a launching call still pays one clone inline: the refill's, or the
fallback's when the buffer is empty or stale.

BASE INVALIDATION IS A CORRECTNESS HINGE, not a nicety. A pre-cloned engine is a
SNAPSHOT of the base's plugin decls and env. The base is a swap point tagged with a
`generation`; a plugin-registry change - a `plugin add/rm` on the stateful lane, OR
an external edit from the user's own shell, both seen through the file mtime -
rebuilds the base, bumps the generation, and DROPS the stale ready buffer. So the
stateless pool picks up a plugin the stateful lane just registered without a restart.

In-flight evals finish on the clone they already hold, dispatched under the old
state, which is fine; only the buffer and future clones pick the change up.

### fn semaphore
The permit rides IN the eval thread and releases when the thread finishes, which is
deliberate: a hung thread keeps its slot OCCUPIED - bounded and visible - rather than
leaking a thread while freeing the slot for another hang.

### fn base / fn ready
Every lock goes through these, never `expect`: a panic under either lock would
otherwise poison it and panic every later dispatch. A poisoned base is rebuilt with its
generation bumped, so every ready clone of the old one drops at the next take; a
poisoned buffer is emptied. Either costs one rebuild, and the poison is cleared.

### fn refill_ready
The clone, which is the cost, happens OUTSIDE the ready lock.

### fn refresh_base_if_stale
Affordable on every dispatch only because the stat is roughly sub-microsecond, and
it rebuilds solely on an actual mtime change.

### fn take_clone
Holds the BASE lock across the pop so the generation read and the fallback clone
stay consistent with a concurrent `refresh_base_if_stale`. Without that the pop
could take a clone whose generation had just been superseded.
