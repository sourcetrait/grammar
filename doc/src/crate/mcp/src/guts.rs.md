# guts.rs

The test-only-`pub` convention's landing site. `TestServer` wraps a `NuSh` plus a
tokio runtime and drives the real tool handlers in-process, returning each tool's
envelope read from the NUON text the wire carries as a NuVal, so the integration
tests exercise the handlers without spawning the binary and assert on the values
nushell's own kinds keep: a duration stays a duration and an int never a float.

The process-global constraint is the thing to hold before writing a test here.
The engine starts once a process and the homes are `LazyLock`s that read the
environment at first access, so every `TestServer` in one test binary shares one
namespace: one id and namespace, one data home, one equip git repo, and one host
nom, the process's (`process_identity`, `server/identity.rs`), which its engine
exports. Per-file isolation is therefore free
(each `tests/*.rs` is its own process) while within a file tests must use unique
equip names and must not rely on a pristine namespace. A test that genuinely needs
an empty namespace belongs in its own binary or in the system crate.

The tests within a binary still run in parallel: each file's `TestServer` is one
`module_with!` instance its tests share, the equip repository's write lock serializes
their commits, and a file whose tests read or write one shared state runs them as
named steps of a single `Stepper` test rather than asking for `--test-threads=1`.

## fn sweep_dead_test_namespaces
Each test binary gets its own `grammar_inproc_<pid>` root holding a full equip git
repo and a keypair, and a test harness returns from `main` with no hook to hang
teardown on: statics never run `Drop`, and there is no atexit here. Left alone
these accumulate one per run forever - a real sweep found 128 of them, roughly
20 MB.

So each run sweeps the dead ones on the way in, which bounds the litter to at most
one namespace per currently-running test binary instead of one per run ever. A
leftover whose pid is gone from the process table (`sourcetrait_process`) cannot be
in use by anyone, and that is the whole liveness test. On macOS the table holds the
user's own processes alone, and the temporary directory is the user's own, so every
namespace there belongs to a process the table can show.

A table that cannot show this very process proves nothing dead, so the sweep is then
a no-op rather than a guess: without a liveness check it could delete a live
concurrent binary's namespace. Best-effort throughout, since a failed sweep must never
fail a test. Pid reuse only defers a removal by one round.

## fn namespace_config
Built over `Config::default()` rather than field-by-field so the harness picks
up every embedded default, `[channel]` included, without restating them; only
the id, namespace, work dir, and remotes file are test-specific. It reads no
home, so building it before the environment is set is safe.

## fn ensure_test_env
The `unsafe` block is sound because the `Once` serializes it and it runs before
the first read of the homes - they are only touched inside `init_engine`,
`ensure_substrate`, and the cache paths, all of which are reached after this.
That ordering is the entire safety argument, so a future caller reaching a home
earlier would break it.

The UENV homes are set beside the XDG ones because the host reads a home's UENV
variable first (`home.rs`): a runner whose login sets the UENV tree would
otherwise put every in-process test's state in the user's real data home.

The engine is initialized here, once a process, since its parameters are
process-wide: `init_engine` resolves nushell's config dirs as the host does at
startup, which every engine reads, so the lint and template drivers below call
this first, because they build an engine without a `TestServer`.

## fn test_config
Each server takes a config of its own, equal to every other the binary builds,
since a controller holds the config it starts on.

## struct TestServer

### fn new
The runtime is sized like the host's own, and for the same reason: the body lint
and the rig validator parse inline on a worker, and a pathological module graph
overflows the 2 MB default and aborts - which in-process would take the test
binary down rather than just a host.

A fresh `TestServer` per test builds a fresh `NuSh` on a controller of its own, the
face's systems and the engine, which is roughly the spawned binary's startup cost minus
the process spawn. That is what makes the in-process tier fast and cleanly isolated - each test gets
its own stateful engine on its own runtime, with no subprocess. It leaves the serve
path's supervision and its namespace lock out: the binary's servers share one
namespace, which a lock held by the first would refuse to the second, and an operator
would sample the test process.

### fn interface
The whole tool, parse and dispatch, for a test of the contract or of a command as an
agent sends it. The typed helpers beside it call each command's method straight, so a
test of a command's logic passes typed values and never quotes a literal; the parse has
its own tests.

### fn equip
An action no command has goes to the parse, so a test of an unknown action meets the
refusal an agent would.

### fn channel_verified
There is deliberately no channel-open helper on this harness. `interface channel open
async` binds a real socket and presents a CA-issued leaf, so an in-process test would be
asserting the machine's certificate installation rather than anything in this crate; it
is exercised live on the test channel instead. A sync open binds nothing, so a test
drives one through `interface` itself.

### fn tools
The router's own listing, each tool serialized as `tools/list` carries it, so a
test holds a description or the absence of an output schema as a client sees it.
It stays JSON, the JSON-RPC's own.

### fn rig_index
Decodes the NUON index and carries it as a NuVal, so a test checking
`source_path` does not have to know the on-disk format.

## fn nuon
An expected value is written as the NUON text it is, so a test reads as the
envelope it compares.

## struct SupervisedSystem
mcp's controller as the serve path starts it, the namespace's lock taken first and
held by the controller, supervising, the operator with the engine, for the one test
of its whole lifecycle; that test is a binary of its own, since the lock is the
namespace's.

### fn host_lock_held
Tried, not read: `File::try_lock` on a fresh open of the file is the namespace
lock's own lock on each platform, `flock` on Unix and `LockFileEx` on Windows,
and it conflicts with the controller's even within one process, because a
`flock` belongs to the open file description and a `LockFileEx` lock to its
handle; the attempt releases at once when it succeeds, the file closing. A
read-only open serves: a `flock` takes a descriptor of any mode, and
`LockFileEx` a handle with read access.

## fn clear_config_pins
The pin registry is process-global, so a test that pins has to be able to put it
back for the next test in the same binary. Unlike the namespace on disk, a pin is
not isolated by using a unique name.

## fn rig_block
It lives here rather than in a test file because the in-process namespace is
shared per test binary: a bare `contains` against the whole signature block can be
satisfied - or falsified - by a rig some other test in the same binary committed.
`rig_call.rs` had exactly that trap waiting, asserting a helper named `util`
absent while `sourcetrait/importable:util` existed.

## fn lint_definition
Parses as `analyze_definition` does and walks the same block, its parse errors
aside, so a rule test exercises the walker alone: a definition `use`-ing a file
that does not exist still has its path judged. Builds a full-shell `ParseEngine`
per call. The plugin-registry read plus the heavy engine build is precisely why
the lint tests are integration rather than unit, and the same is true of the
template drivers below.

## fn lint_engine_reuses_until_the_registry_moves
Identity is the only thing that catches this regression. A `LintEngine` that
rebuilt on every `current()` would still behave correctly and would still pass
every behavioural test, while quietly paying a full command-context build on each
lint and each commit.

The other direction - that a changed registry does rebuild - is proven on the live
channel rather than here, because faking it means writing to the user-global
`plugin.msgpackz` that every process on the box shares.
