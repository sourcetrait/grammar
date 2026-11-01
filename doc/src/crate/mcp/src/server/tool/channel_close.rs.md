# channel_close.rs

## const CLOSE_PLANNED
Sending a real close frame is what lets the agent tell a planned teardown from
a host that simply died, which arrives as a bare 1006.

## fn channel_close
Idempotent, like `interface equip uninstall`: closing what is already closed is
the requested state rather than a failure.

A deliberate close removes the inbox. The caller is declaring it is done with
the channel, which is what makes dropping that channel's attachments
intentional rather than inferred. Scoped to this path on purpose: the shutdown
close, the verify-timer expiry and the unverified-emit teardown are all
host-initiated, and none of them is the caller saying it has finished.

The removal is best-effort because the close has already happened and this
command cannot fail. The path is the host's own, made by `interface channel
open` in the transient base and never caller-supplied, which is what makes a
recursive remove bounded here rather than a hazard. `remove_channel_dir`
forgets the path as it removes it, so the host's end never removes a directory
that took the name afterwards. The peer links hold the inbox gone before it is
removed (`channel_open.rs`'s `tell_inbox`), so a file they land after fails
rather than recreating it.
