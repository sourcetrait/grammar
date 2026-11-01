# system.rs

The operator as a subsys system: its own tick, the observations its manager passes, and
the emergencies it sends back, each a singular packet. The namespace's lock is its
controller's, taken before the engine, never the operator's (`crate/scratch`'s
`lock.rs`).

## struct OperatorSystem
### field probe_rx
The slow probes answer HERE, on the loop's own channel, rather than being awaited
inside a tick. A probe awaited in the loop stalls everything the loop does - the
reaping, the pins' expiry, the observations, and the manager's stop - for as long as a
wedged `df` or `nvidia-smi` hangs.

## struct OperatorSysParams
### field test
A `--test` host is the machine's second host, so it samples only while its Channel is
open; a production host samples whether or not anyone listens.

## struct OperatorSysPaths
The namespace's homes. The secrets dirs are the keypair's, the namespace's one secret,
which the operator never reads.

## fn on_tick
A due probe runs on a blocking thread and sends its answer to the loop. The watchdog
starts a kind again only after its last answer, so a probe that never returns costs one
blocked thread, never one a tick.

## fn run
The interval's first tick completes at once and is spent before the loop, so the first
sample comes one interval in, as the watchdog always sampled. A late tick is delayed
rather than burst, so a busy loop never samples twice back to back.

## fn OperatorHandle::observe
Singular, no answer awaited: the manager's observer never waits on the operator, and an
emergency the observation confirms comes back on the handler.
