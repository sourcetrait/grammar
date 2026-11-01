# ssh.rs

A worker reached over SSH. The MCP connects to the host with russh, holds the
host's key to the pinned ones, authenticates with its key file, and runs the
worker's command line on an exec channel; the channel's data is the session's
reader, its writer the session's writer, and its stderr the MCP's log under
the engine's alias. No system ssh, agent, forwarding, or PTY is involved.

## const PIPE_BYTES
The worker's output the pump holds before the session reads it; past it the
pump waits, and the channel's window holds the worker back.

## const LINE_MAX
## const TAIL_LINES
A stderr line longer than `LINE_MAX` is logged in pieces; the last
`TAIL_LINES` are kept to explain a worker that ends before its reply.

## const CLOSE_FLUSH
How long a close waits for its frame to leave and the channel to end.

## struct RemoteSession
The session, and the connection it rides, held so it lasts.

### fn close
The session's close is written and its stream ended, then the connection is
ended. The worker ends with its session either way, on the close or on its
stdin's end.

## fn open
The pinned keys and the key file are read first, so a bad one fails before a
connection is made. The connection, the authentication, and the exec run
within `AUTH_DEADLINE`. A session that doesn't open ends its connection and
waits for the pump, so a worker that exited before its reply is told as what
it did: its exit and its last stderr lines.

## fn preferred
russh's own key exchanges, ciphers, and MACs, which carry no SHA-1, and its
host key algorithms less `ssh-rsa`, whose signatures are SHA-1's.

## fn connect
TCP, the SSH handshake, the public key authentication, the session channel,
and the exec, each failure its own transport error: the host unreached is
`connect_failed`, a key not pinned `host_key_rejected`, the key refused
`authentication_failed`, and a channel or a command refused `unavailable`.

## fn rsa_hash
An RSA key signs with the strongest SHA-2 hash the host names, and a host
naming none refuses it; any other key carries its own algorithm.

## struct Pinned
The connection's handler. The host's key matches a pinned one by its key data,
so a comment never matters, and a refusal keeps the presented key's SHA-256
fingerprint for the error.

## struct Witness
What the pump saw beside the worker's output: the host refusing the command,
the worker's exit, and its last stderr lines.

### fn explain
A session lost before its reply becomes `unavailable` with what the worker
did, where the pump saw it; any other refusal stands.

## fn pump
The channel's messages in order: data into the session's reader, stderr into
the log, the end of the worker's output ending the reader's, and its exit
logged. The pump outlives a reader that's gone, still logging.

## fn log_lines
Each whole stderr line logged as `grammar: <alias>: <line>` and kept for the
tail; at the channel's end, a partial last line too.
