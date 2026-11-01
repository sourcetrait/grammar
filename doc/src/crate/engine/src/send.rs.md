# send.rs

## fn prepare_send
A body's remote send as its host sends it on, its files read and its event
rendered where they are: the MCP for a local eval, the worker for a remote
one, which carries the result to the MCP inline, so no host reaches another's
files. It is the engine's because both hosts take the engine and neither the
other, and it runs off the runtime's workers, a blocking read of the caller's
own files. The files are read before the event renders, the order a send's
refusals have always had.

## fn read_files
Each {src, dest} row is read before the send's id is answered: a src the sender
can't read is the agent's own problem and surfaces at the call, not as a later
Unsent. Each dest is held to `safe_dest` here too, so a bad dest fails the send
outright rather than being caught only on the receiver.
