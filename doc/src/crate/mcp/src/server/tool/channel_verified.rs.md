# channel_verified.rs

## fn channel_ready
THIS IS THE AUTHENTICATION. The same agent that drives the MCP over stdio proves it owns the
CLAIMING connection - which is all the Monitor's `{url, protocols}` input can support, since it
carries no headers, no client cert and no auth hook.

So a claim has to EXIST to be owned, which is why `not_claimed` is a distinct refusal rather
than a success on an empty channel.

A sync channel proves the same thing another way: the greeting came back through the
agent's own stdio session, by `interface channel sync`, so the first sync is its claim.
The refusal names the mode's next step.
