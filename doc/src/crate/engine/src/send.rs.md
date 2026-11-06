# send.rs

## fn prepare_send
A body's remote send as its host sends it on, its files read where they are:
the MCP for a local eval, grammar-ssh for a remote one, which carries them to
the MCP inline beside the send's data, so no host reaches another's files. It
is the engine's because both hosts take the engine and neither the other, and
it runs off the runtime's workers, a blocking read of the caller's own files.

## fn read_files
Each {src, dest} row is read before the send's id is answered: a src the sender
can't read is the agent's own problem and surfaces at the call, not as a later
Unsent. Each dest is held to `safe_dest` here too, so a bad dest fails the send
outright rather than being caught only on the receiver.
