# run.rs

## fn run_main
The session's end is logged on stderr and the process exits 0; a failure is
logged behind `grammar-ssh: ` and exits by its code, so a script tells a held
namespace from a bad workdir without reading text. stdout carries nothing but
the wire.

## fn wire
The wire is stdout's own descriptor or handle, duplicated, and the standard
output then points at stderr, so nothing else in the process, a stray print or
an external that inherits the standard output, can write into the stream: what
would corrupt a frame lands in the MCP's log instead. The duplicate is the
wire's one copy, and no external inherits it: one holding it would keep the
session's stream open past the worker's exit, since the stream's reader sees
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

## fn serve
The environment is read once as data, an entry that isn't UTF-8 left out. The
start comes before the hello, so a worker answers the hello with its namespace
held, its engine up, and its scratch made, or with why it couldn't. The
shutdown runs whatever the session's outcome, so a session that never opened
still releases its lock and removes its scratch.

## fn start_dir
The workdir named, or the process's own directory when none is, read before
the start, so a directory that can't be read is refused through the hello as
a bad workdir.

## fn enter
The engine seeds `$env.PWD` from the process's working directory, which a
session's sshd leaves at the account's home. An MCP's client starts it in the
agent's directory, so a local eval starts there; the worker moves into its
workdir before its engine starts, so a remote eval starts in the workdir the
MCP's entry names, as if it ran locally. It is one process-wide change, made
before any eval thread exists, which no eval makes again, the engine merging
an eval's environment without nushell's chdir (`crate/engine`'s `embed.rs`,
`merge_env_no_chdir`). The path is the real one, so on a machine whose
temporary directory sits behind a link, as macOS's does, `PWD` is the resolved
path while `EQUIP_WORK_DIR` keeps the given one.

## fn refuse_with
A worker that couldn't start still reads the MCP's hello and refuses it with
its failure's kind and text, so the MCP reports a held namespace as
`remote::engine::unavailable` naming `runtime::namespace_held`, not as a lost
session. A runtime of its own is made for that, the worker's having never
started.
