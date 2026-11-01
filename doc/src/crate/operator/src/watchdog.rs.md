# watchdog.rs

The watchdog's state and its rules, driven by the operator's system: its tick, its
manager's observations, and its probes' answers (`system.rs`). Evals run on dedicated
threads OFF the runtime, so the system stays schedulable even under TOTAL eval
saturation - the emergency path cannot be hung by what it reports.

CLASSIFY-FIRST: it never kills or recovers. A legitimate heavy transform looks identical
to a runaway, and that false positive is the one to avoid.

PLATFORM-GATED throughout: the /proc readers are Linux-only and return None elsewhere,
and nvidia-smi is RUNTIME-probed, so a GPU-less box simply skips it. The parsers are pure
and unit-tested over fixture strings; the readers are thin wrappers, which is what makes
that split testable at all.

## const SAMPLE_INTERVAL
Public because the manager observes at the same cadence the watchdog ticks at, and
bounds its wait for the engine's health by it.

## const HUNG_GRACE_MS
Because tree-kill fires at timeout, an eval merely BLOCKED on an external is unblocked -
its thread returns, `finished` flips, and it is pruned. Only a pure-Rust hang survives to
confirmation, which is exactly the distinction the grace exists to draw.

## const LEVEL_SUSTAIN
RESOURCE LEVELS ARE THE AGENT'S CALL, NOT THE HOST'S. Heavy load is something the agent
decides how to react to, and it needs ONE warning over a long period to make that
decision - not a stream of them.

A momentary excursion is not an event, so a level must hold for a full minute. Any dip
below the line restarts this clock.

## const LEVEL_REWARN
Ten minutes is the minimum that would matter; dozens of minutes is the intent. Thirty is
chosen because AN INFERENCE PROJECT RUNS 30 TO 60 MINUTES and holds resources high for
all of it - which is NORMAL work, not a fault. At the ten-minute floor such a run would
warn six times about a condition the agent already knows about and chose; at thirty it
warns about twice, which is enough to notice a genuinely sustained level without talking
over the work.

The long floor is also what removes the need for a separate CLEAR threshold: a value
hovering AT the line cannot produce an event per oscillation.

## fn total_ram_kb
A FRACTION OF THE MACHINE rather than an absolute figure, so the same default means the
same thing on a box with different memory instead of silently ageing.

## const DISK_SAMPLE_EVERY
Five minutes rather than the two-second tick, because `df` is slow and can stall on a
wedged mount.

## fn parse_df
The mount point is everything after the FIFTH column, since a mount path may contain
spaces while the five numeric-ish columns before it may not.

## struct DiskWatch
THE BASELINE DECIDES WHAT WE WATCH. A filesystem already at or above the line when the
host starts is a PRE-EXISTING CONDITION, not something to report: this box carries a
4 KiB `/run/nvidia-ctk-hook...` pseudo-mount sitting at 100% by design, and a naive
threshold rule would warn about it forever. Only those below the line at startup are
enrolled; they warn if they later cross it.

Worth knowing what this watch happens to cover: the Channel's inbox sits in the runtime
directory, a small tmpfs where logind makes one, so the attachment lane's own failure
mode is inside this gate.

## fn probe_disk
Blocking, run on a blocking-pool thread by the system, because a wedged mount must take
neither the async runtime nor the system's loop with it.

## struct LevelGate
Pure over its own state and takes an INJECTED `now`, like `scan_hung`, so it is
unit-tested without sleeping.

## const CLK_TCK
A non-100 kernel skews only the LOGGED percentage, never any action - nothing here reacts
to the number, so the approximation is bounded to the report.

## fn scan_hung
Pure over the engine's reading so it is unit-testable with synthetic rows, and it returns
the stateless count separately because that is what the derived Critical is measured
against.

## fn rising_edges
AN ABSENT READING IS NO READING. An observation the engine didn't answer leaves the active
set as it was, so a hang it reported stays active, and its next answer raises nothing
twice. Diffing an absent reading as an empty one would clear every condition and re-raise
each on the next answer.

## fn parse_cpu_ticks
Fields counted AFTER the last ')' since the comm field can contain spaces and parens, so
utime (field 14) is index 11 and stime (field 15) is index 12.

## fn probe_vram
Blocking, like `probe_disk`, and for the same reason: a hung `nvidia-smi` must stall
nothing but its own thread.

## struct DueProbes
A kind is due on its cadence AND only while its last probe has answered, so a probe that
never returns costs one blocked thread rather than one a tick.

## struct Watchdog
### fn online
A PRODUCTION host samples whether or not anyone is listening: the log is the durable
record, and sampling while idle is intended. Under `--test` it FOLLOWS the channel
instead, as the manager last observed it, and follows rather than latches, so closing and
re-opening a channel takes sampling down and brings it back.

The waste this removes is MEASURED, not theorised. A second host on the box - a
`grammar_test` host connected with its channel closed - shells `nvidia-smi` every 16
seconds and logs VramWarnings about a card it has no stake in, none deliverable (the
announce is verification-gated while the log is not). Across a long training burn both
namespaces logged the same 57 readings with timestamps within 18 ms, because a watchdog
runs per host and the probe reads the whole card. `--test` is precisely the flag that
says this host is the second one.

### fn tick
HOUSEKEEPING RUNS UNGATED, above the sampling gate. Harvesting adopted zombies and lapsed
pins is owed whether or not anyone is watching levels; skipping it would trade a little
wasted sampling for a real leak. Neither raises an Emergency.

`OrphanReaper::reap` lives on this tick for the same reason everything else does: the
tick runs off the eval threads, so a /proc scan cannot be starved by eval saturation.

CONTINUITY STATE IS RESET while offline - `prev_cpu`, the active keys, the four gates and
the `DiskWatch` - so resuming re-establishes rather than inherits. Without that the first
sample back would report a CPU average spanning the whole offline gap, and a gate could
fire on a sustain window covering a period nobody measured. A reading we never observed
is worse than no reading. The probes in flight are NOT reset: they track threads that
still run.

### fn observe
EDGE-TRIGGERED: the hung and Critical conditions are raised once when they first arise,
via a rising-edge diff of the active-key set between the engine's readings, so the log
captures distinct EVENTS rather than a per-second time series of one ongoing condition.
The resource levels reach the same outcome by a different route, through their own gates.

The engine's conditions are classified as each observation ARRIVES rather than on the
tick, so no reading waits for a tick and no tick reads a stale one.

### fn vram_sampled
A probe answering while offline frees its slot and feeds no gate: its reading belongs to
a period the gates were reset for.
