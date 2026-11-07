# job.rs

An await job: prepare runs within the call, execute runs as the job, status at
each beat, and stop at an interruption, each an eval of its own carrying the
suite's module inline, its text also cached at `await/<nonce>/suite.nu` as the
job's record. What the long runner did by hand, a worker job and a supervisor
job with a trace, the engine does here (User).

## struct PhaseTimeout
A definition's bound in nanoseconds, so the literal its `timeout?` takes and the
milliseconds the engine waits are one value. `0` is execute's alone and means
unbounded.

## enum StopReason
What a Stop event names: a timeout, `stop` for a stop by hand, or the teardown
(User).

## struct AwaitJob
What a running job holds: its call's nonce, its directory, its module's text,
the PREPARED record's literal each definition takes, its timeouts, its id in the
jobs table, its start, execute's interrupt flag, tracker, and thread end, and
the queue its events are delivered from. The queue's one sender is the job's,
so the queue closes once the job's last holder lets it go.

`settled` is taken once, by execute's end or by the first interruption, so a job
sends one Result or one Stop and stop never runs twice. `done` stops the beats;
the two `Notify`s each have one waiter, so `notify_one`'s stored permit covers a
notice that comes before its waiter.

### fn trace
One NUON line an event under the job's directory. nuval's raw style is one line
but leaves a newline inside a string raw, so the line escapes it, as the host
does for a packet. The trace keeps an event the Channel couldn't take.

## fn serve_await
Every refusal comes before anything runs: the contract, the timeouts, the
heartbeats, and the data. prepare takes the call's `timeouts` and `heartbeats`
as the tool got them, null where absent, passed as the template passes an absent
argument, and runs on the pool in flight under the call's nonce, so the teardown
cancels it as any call's.

## fn start
The job's id comes from the jobs table, and the PREPARED record carries it, so
the record is built after the add and the add is undone if building fails.
execute's eval takes a clone of the stateless base with the job as its current
job, so `job id` answers the job's id; it takes no pool permit, since a job
holding a slot for hours would starve the pool. The in-flight row stays under
the call's nonce, so the teardown reaps the job's thread as it reaps a call's.
The job's record goes in the call's answer alone; no event carries it (User).
The job's sender starts before the job is built, so every event it queues has a
task to deliver it.

The job's thread job is wired as nushell's `job spawn` wires one (nu-command
0.116.1, `job_spawn.rs`): its mail channel's sender in the jobs table, its
receiver execute's current job's mailbox, so `job send <id>` from any eval
reaches execute's `job recv`. `make_tracker`'s tracker drops its receiver, and
with it a send to the job was lost without a word, nushell's `job send` dropping
the disconnected channel's error.

No job starts once the teardown has begun (User): the job joins the engine's
jobs under their lock only while `stopping` is unset, and `stop_jobs` sets it
under the same lock as it takes its list, so a job either joins the list the
teardown stops or is refused. An await whose prepare returns during the teardown
is refused so, its job's add undone; its prepare ran outside any job, as every
prepare does. The refusal is `internal` at `await::start` rather than a variant
of its own, which would change the wire's schema for a call no one hears, the
host being the one stopping.

## fn run_execute
execute's end, its timeout, and a stop's notice race. Its end sends the Result
when it settles the job; its timeout runs the stop; a stop's notice ends the
watch even when a killed thread never returns.

## fn run_beats
A gap is counted from the previous beat, the first from the start, the last
repeating, so the first Status comes as the first gap ends and none before it
(User). The schedule is calculated from the await's start on the monotonic clock
(User), each beat's time the start plus the gaps before it, so neither a
status's run nor its permit's wait moves a later beat; sleeping each gap after
the last status and its send drifted by both and by the send. A Status is queued
for the job's sender and the beats go on, so a delivery the host holds holds no
beat: awaiting the send let a slow one skip every beat whose time passed while
it was held. A status running past the next beat's time lets that beat run as
soon as it ends, and any time it passed beyond that is skipped, so beats never
overlap. No beat starts once the job is done.

## fn catch_up
The beat after the last one run: its time where that has not come, else the
latest time already passed, which runs at once. Stepping one gap a pass was
bounded only by the time elapsed, so 1ns gaps behind a one-second status took a
billion passes with no await between them, pinning a runtime worker. The list's
own gaps are stepped, at most its length, and the repeating last gap is crossed
by division, the latest time passed found as now less the remainder, so no
product of gaps is ever added to the clock. Its tests hold it to the stepping it
replaced.

## fn stop_job
stop runs while the job still runs, so it can tear down what the job holds, then
the job's thread is cancelled and its externals' trees killed, then the Stop
event goes, so the event means the job is gone. The thread is registered for the
hung watch, as a timed out call's is.

## fn stop_jobs
The teardown's: every job's stop at once, each bounded by its own timeout. A
teardown's Stop event is traced alone: the host it would reach is the one
stopping, the MCP at its shutdown or grammar-ssh's session at its end, and an
ask of it could wait on a system that answers no more. The teardown sets the
engine's `stopping` as it takes its list, under the jobs' lock, so the list is
every job there will be.

## fn running_job
A job is over once it is done, its end or its stop begun, so the commands never
name a job its own end is already taking out.

## fn await_list
The jobs by their ids, which the jobs table hands out in order, so the rows run
in the order the jobs started; each row's `started` is its PREPARED record's
(User).

## fn await_status
status run once, as a beat runs it, its value or its error's rows in a Status's
data; the answer is the agent's alone, so no packet goes (User).

## fn await_stop
The stop by hand, `interface await stop`'s, answering once the stop has run.

## fn event_data
A Status's, a Result's, or a Stop's data: the call's nonce as `await`, the
event's own field, then the value under its field or its error's rows under
`error` (User).

## fn emit
The event is traced, then queued for the job's sender, so neither the beats,
execute's watch, nor a stop waits on the host. The trace holds the events in the
order they were queued, the order they are delivered in.

## fn send_events
A job's events delivered one at a time, each asked of the host from a blocking
thread, since the host link's ask blocks; a refusal is logged and the next one
goes. One sender keeps the events' order, its Statuses', its Result's, and its
Stop's, and a slow delivery delays only the deliveries after it. The queue
closes once the job is gone, and the task drains what is left before it ends, so
an event queued as the job ends is still delivered: a status running as its job
ended reports after the Result, never torn down with the job (User).

## fn suite_timeouts
The record's four fields are each required, null taking the default, as no type
definition holds an optional field (User); `0ns` is execute's alone.

## fn heartbeat_gaps
Two intervals at least where given (User), each positive; the default schedule
is the document's.
