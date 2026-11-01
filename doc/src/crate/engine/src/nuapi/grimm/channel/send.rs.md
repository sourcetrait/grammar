# send.rs

## struct GrimmChannelSend
The Channel is a notification lane, not a serialization lane. `event` says "state
changed, here is a tiny summary"; the agent fetches the actual data itself. That
premise is why the signature has two slots rather than one: an author with
something bulky puts it in `attached`, which the host writes to the inbox and
names on the wire rather than carrying. The contract is structural, the ergonomic
path being the next argument, so nobody has to be talked out of putting data on
the wire.

It returns the message id as `{event_id}`, the record its signature declares, so
an author can correlate what it sent with what the agent later fetches.

### fn run
The arguments are checked here, the event and any attachment a record or a table,
and the sending call's origin read from `$env.NONCE`. The send then crosses as
data to the engine's controller (`HostAsk::ChannelSend`), which hands it to the
Channel's system: the `mcp/` refusal, the phase, the spam gate, the id, the inbox,
the spill, and the emit, in the order `crate/channel`'s `plan_send` gives its
reasons for. The engine keeps the one act on its own state, stopping the offender,
which a stop verdict asks of it.

## fn stopped
The verdict's answer is the refusal the send returns; when the stop begins an
episode, the action taken is reported back first (`HostAsk::Stopped`), so the
error's emergency names what was done. The report is best effort: the send is
refused either way.

## fn shell_error
nushell's split: the title names the command and that it failed, and the label
carries the host's reason. `NuValError::short` renders both on the envelope's one
line, so a reason in each read twice. A body catching the error reads the title as
`$e.msg`, the reason in its rendered labels.

## fn stop_offender
A stop verdict runs it when a send crosses the hard threshold, and the action it
returns rides the emergency.

Two levers, because the offender has two shapes and neither lever reaches both.

`Signals::trigger` reaches a spam loop whatever it runs under: such a loop invokes
a decl over and over, so it polls at every one of those boundaries and bails.

But a job that outlived the eval that spawned it has no `in_flight` entry left,
`InFlightCleanup` having removed it the moment the dispatch returned, and the jobs
table is the only place it is still reachable. A spawned closure reads its own
JobId here, so `current_job` names the actual offender rather than its parent.

Proven live: a detached `job spawn` whose loop caught every throw and had 196
iterations left still stopped at the fifth send, and the jobs table went empty.
That is the case the throw alone cannot cover, and the table emptying is the half
`Signals` could never do. The `action` string records which branch ran, which is
also how it is verified.

Only the processes of a killed job can fail to die; the table entry goes either
way, and an external the job left behind is the tree-kill machinery's to reap.
