# remote_channel_close.rs

## fn remote_close
The peer links' system removes the link and cancels its drivers; the link's own
task observes the cancel and tells the system, which reports
remote/channel/Disconnected - so a close and the Disconnected notice share the
one lifecycle path rather than each reporting separately.

An alias with no established link may name a listener still awaiting its peer:
its close answers once the accept has let its port go, and the accept task
reports Disconnected {remote}. not_open only when the alias names neither.
