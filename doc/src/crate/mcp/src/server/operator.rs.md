# operator.rs

The operator's side of mcp's controller (`controller.rs`): its handler, which
hands its emergencies to the policy and hears its end, and its observer, which
feeds it the engine and the Channel.

## struct OperatorWatch
The handler the controller hands the operator's system. The operator's
emergencies arrive as packets no request waits on, so the control task hands
each to this handler, which answers it with the controller's policy
(`emergency.rs`) on the control task itself. The control task awaits each
before it takes the next, so the emergencies are logged and announced in the
order the operator raised them; the announcement is a request to the Channel's
system, never to the operator's, so it never waits on the control task that
awaits it. Its status changes and its end are logged, the shutdown's own
unsaid (`controller.rs`'s `log_status` and `log_closed`).

## fn engine_health
Bounded by one sample's interval: an engine that cannot answer within it gives
that observation no reading rather than stalling the observer, and the operator
keeps its active conditions across an absent reading.

## fn spawn_observer
One observation an interval, the operator's own cadence. The Channel's phase
rides with it because the Channel is mcp's: a `--test` host's operator samples
only while it is open. The phase is read from the face's Channel the controller
hands it, and a Channel's system that doesn't answer reads as closed. A send
that fails means the operator's system is gone, and the observer ends with it.
Its handle is the controller's, which aborts it first when it stops.
