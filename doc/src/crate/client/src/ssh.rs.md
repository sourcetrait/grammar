# ssh.rs

grammar-ssh reached over SSH. The MCP connects to the host with russh, holds the
host's key to the pinned ones, authenticates with its key file, and runs
grammar-ssh on an exec channel, through the account's login shell on Unix; the
channel's data is the session's reader, its writer the session's writer, and
its stderr the MCP's log under the engine's alias. No system ssh, agent, or
PTY is involved, and no forwarding but a jump's: an entry naming a jump has its
session ride a direct-tcpip channel of the jump's own session.

## const PIPE_BYTES
grammar-ssh's output the pump holds before the session reads it; past it the
pump waits, and the channel's window holds grammar-ssh back.

## const LINE_MAX
## const TAIL_LINES
A stderr line longer than `LINE_MAX` is logged in pieces; the last
`TAIL_LINES` are kept to explain a grammar-ssh that ends before its reply.

## const CLOSE_FLUSH
How long a close waits for its frame to leave and the channel to end.

## struct RemoteSession
The session, and the connection it rides, held so it lasts.

### fn close
The session's close is written and its stream ended, then the connection is
ended, then the jump's, whose channel the connection rode, so the jump's
session ends with the engine's. grammar-ssh ends with its session either way, on
the close or on its stdin's end.

## fn open
The pinned keys and the key file are read first, the jump's with the engine's,
so a bad one fails before a connection is made. Both hops' connections, their
authentications, and the exec run within `AUTH_DEADLINE`. A session that
doesn't open ends its connections and waits for the pump, so a grammar-ssh that
exited before its reply is told as what it did: its exit and its last stderr
lines.

## fn preferred
russh's own key exchanges, ciphers, and MACs, which carry no SHA-1, and its
host key algorithms less `ssh-rsa`, whose signatures are SHA-1's.

## struct Login
One hop's login. Where a jump is taken, each hop is named in its refusals by
its place, `the jump `<alias>` at <host>:<port>` or `<host>:<port> through the
jump `<alias>``, so a refusal says which hop failed under the transport's own
kinds; a direct session's refusals read as they did before jumps.

## fn connect
TCP, the SSH handshake, the public key authentication, the session channel,
and the exec, each failure its own transport error: the host unreached is
`connect_failed`, a key not pinned `host_key_rejected`, the key refused
`authentication_failed`, and a channel or a command refused `unavailable`.

Through a jump, the jump's TCP, handshake, and login come first, then a
direct-tcpip channel from its session to the engine's address, `127.0.0.1`
port 0 its originator, and the engine's own handshake runs over that channel,
its host held to its own pinned keys, then its login and the exec. The jump's
session carries the channel alone and starts no grammar-ssh; a channel the jump
refuses is `connect_failed` on the jump's hop.

## fn handshake
Generic over its stream, a TCP connection's or a jump channel's, so both hops
share one handshake and one host key check.

## fn end
The engine's own disconnect crosses the jump's tunnel, and russh's session takes
a channel's data and its own messages from two queues in no set order, so a jump
disconnected at once could end before the tunnel's last bytes left: the
engine's host logged a dropped connection, never the close, as the macOS VM's
loopback sshd showed. The jump's host closes the tunnel only after everything
sent on it, so the jump waits for that close, up to `CLOSE_FLUSH`, then
disconnects.

## fn rsa_hash
An RSA key signs with the strongest SHA-2 hash the host names, and a host
naming none refuses it; any other key carries its own algorithm.

## struct Pinned
The connection's handler. The host's key matches a pinned one by its key data,
so a comment never matters, and a refusal keeps the presented key's SHA-256
fingerprint for the error. It tells `end` when the host closes a channel: a
jump's one channel is its tunnel.

## struct Witness
What the pump saw beside grammar-ssh's output: the host refusing the command,
grammar-ssh's exit and its status, and its last stderr lines.

### fn explain
A session lost before its reply becomes `unavailable` with what grammar-ssh
did, where the pump saw it; any other refusal stands. A grammar-ssh its host
doesn't have is `command not found: <command>`, the command as the entry
names it, the words a shell's own user knows.

### fn not_found
The login's shell telling of a missing command, as measured on Fedora and
macOS: bash, zsh, sh, and fish exit 127 with the command named on stderr;
macOS's sh, bash 3.2, exits 126 for a missing absolute path, naming it with
"No such file or directory"; and nu exits 1 with ``Command `<command>` not
found`` and its help line. A 127 not naming the command is a missing shell or
another failure, and a 126 without those words a command found but not run, so
each stands as what grammar-ssh did.

Windows' shells, as measured on Windows Server 2025, each exit 1: PowerShell
with ``The term '<command>' is not recognized``, cmd with ``'<command>' is not
recognized as an internal or external command``, and nu with its own words.
cmd answers a drive path whose folder is missing with ``The system cannot find
the path specified.``, read as a missing command only for a command holding a
`\`: for a bare name the line can only be grammar-ssh's own.

## fn pump
The channel's messages in order: data into the session's reader, stderr into
the log, the end of grammar-ssh's output ending the reader's, and its exit
logged. The pump outlives a reader that's gone, still logging.

## fn log_lines
Each whole stderr line logged as `grammar: <alias>: <line>` and kept for the
tail; at the channel's end, a partial last line too.
