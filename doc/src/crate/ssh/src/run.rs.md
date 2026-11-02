# run.rs

## const WORKER_STACK_SIZE
Every tokio runtime worker gets an eval thread's stack, as the MCP's does
(the MCP's `run.rs`): an interface parse or a definition's check runs inline
on one.

## fn run_main
The session's end is logged on stderr and the process exits 0; a failure is
logged and exits by its code, so a script tells a held namespace from a bad
workdir without reading text. stdout carries nothing but the wire. The runtime
is built before anything starts, as the MCP builds its own; one that can't be
built leaves the hello unanswered, the MCP reading the exit and its stderr.

## fn wire
The wire is stdout's own descriptor or handle, duplicated, and the standard
output then points at stderr, so nothing else in the process, a stray print or
an external that inherits the standard output, can write into the stream: what
would corrupt a frame lands in the MCP's log instead. The duplicate is the
wire's one copy, and no external inherits it: one holding it would keep the
session's stream open past grammar-ssh's exit, since the stream's reader sees
its end only once every copy of the write end is closed.

On Unix the duplicate is made close-on-exec by `try_clone_to_owned`, which
asks `fcntl` for `F_DUPFD_CLOEXEC`, and `dup2` then places stderr on fd 1,
closing the original there. A plain `dup` would leave the copy inheritable,
since a duplicate's close-on-exec flag starts off.

On Windows `try_clone_to_owned` duplicates the handle non-inheritable, and
`SetStdHandle` makes stderr the standard output, which Rust looks up on every
write and for every child that inherits it. The old stdout handle is then
closed, so no copy of the wire is left for a child to inherit: std's `Command`
creates a child with handle inheritance on by default, passing it every
inheritable handle the process holds. Where the old handle is stderr's own it
is kept, since closing it would close stderr.

## fn run
The start comes before the hello, so grammar-ssh answers the hello with its
namespace held, its engine up, and its scratch made, or refuses it with its
failure's kind and text, so the MCP reports a held namespace as
`remote::engine::unavailable` naming `namespace::held`, not as a lost session.
The shutdown runs whatever the session's outcome, so a session that never
opened still removes the scratch and the cache home and releases the lock.

## fn start
The environment is read once as data, an entry that isn't UTF-8 left out. The
lock comes before anything that touches the namespace, so a second
grammar-ssh, or an MCP on the namespace, is refused before its scratch, its
engine, or a plugin; then the scratch, the engine's parameters and substrate,
once, and the controller on them. A failure undoes what the start made, the
scratch removed and the lock released with its descriptor, and leaves the
namespace's state and equips as they were.

## fn start_dir
The workdir named, or the process's own directory when none is, read before
the start, so a directory that can't be read is refused through the hello as
a bad workdir.

## fn enter
The engine seeds `$env.PWD` from the process's working directory, which a
session's sshd leaves at the account's home. The MCP's client starts it in the
agent's directory, so a local eval starts there; grammar-ssh moves into its
workdir before its engine starts, so a remote eval starts in the workdir the
MCP's entry names, as if it ran locally. It is one process-wide change, made
before any eval thread exists, which no eval makes again, the engine merging
an eval's environment without nushell's chdir (the engine's `embed.rs`,
`merge_env_no_chdir`). The path is the real one, so on a machine whose
temporary directory sits behind a link, as macOS's does, `PWD` is the resolved
path while `EQUIP_WORK_DIR` keeps the given one.
