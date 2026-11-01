# system.rs

The Channel as a subsys system, and the handle its face and its senders ask
through.

## struct ChannelSystem
One step at a time, each message's answer computed on the loop, so the state
needs no lock: the order the system takes messages in is the order they act in.

## struct ChannelSysParams
The thresholds the config starts it on, as the wire carries them. A zero window
or rate fails the start, which the config never gives.

## struct ChannelSysPaths
The namespace's homes, as every grammar system takes them. The Channel writes
none of them: its files go to each open's inbox.

## fn ChannelSystem::send
The files are written on a blocking thread, the loop awaiting them, so a bulky
attachment never holds a runtime worker. The Channel's next message waits
behind the write, since the line must not go up before its files exist.

## fn ChannelSystem::send_ups
What a step sent up goes before its answer, each a singular packet the control
task hands its handler before it hands the answer to its waiter. So a send's
line reaches the hub, and a close its frame, before the sender hears back.

## fn on_channel_recv
A notice answers nothing; every request answers once.

## struct ChannelHandle
### field runtime
The runtime the system was started on, captured at its start. An engine's eval
runs on a thread of its own, outside the runtime, and asks from there by
`request_blocking`; a task inside the runtime asks by `request`, since blocking
a runtime thread on its own runtime panics.

### fn notify
A singular packet: the hub's notices and a sender's stop want no answer, and
the system's queue keeps them in their order with the requests around them.

### fn stop
A halt: the system's queue is dropped, and a request after it fails rather
than hangs.
