# common.rs

## struct NuapiCall
The call a host decl acts for, resolved at run time from the stack's
`$env.NONCE`, which every eval puts on its root stack and every job a call spawns
inherits, so a long run's packets still land in its launching call's trace. The
log dir follows from the nonce alone (`call_log_dir`), and the origin with it.
The nonce is random enough that an `execute` call, a rerun, and a call need no
distinguishing prefix.

Not bound at parse time, and that is the point. A decl carrying its call's log
dir served every call but one: the stateful engine persists, and a module an
earlier stateful call parsed keeps its binding to that call's decl, so a later
call's `grimm dbg` appended to the earlier call's trace - measured, the second
call's mark landed in the first call's file and the second call's trace was never
written.

A body that rewrites `$env.NONCE` redirects its own artifacts; the nonce is
checked base62 before it names a path, so it can name only a call's own dir.

### fn stderr_log
Opened to append, as `eval_in_process` opens the stderr log, so this writer and
an external's never overwrite each other.

### fn origin
Host-stamped from the nonce: a body can claim another origin only by rewriting
its own `$env.NONCE`, and only one naming a call whose log dir exists.

### fn append_debug
`to nuon` renders compactly - no pretty-printing newlines between fields - but it
does not escape a newline inside a string value; it emits the byte raw. Verified
rather than assumed: `{msg: "a\nb"}` comes back out of `to nuon` spanning two
lines, which breaks the one-record-per-line invariant a nuonl reader depends on.
So raw newline and carriage-return bytes are re-escaped here.

That substitution is safe precisely because `data` is a record or table and the
render is compact: the only raw newlines the output can carry are inside
double-quoted strings, where `\n` and `\r` are the escapes nushell reads back, so
the line still parses to the original value. Backslashes `to nuon` already escaped
are untouched, since only the newline bytes themselves are replaced.

This is the shape of assumption that needs a live probe rather than a read of the
nushell source - the render's behaviour is not what the source suggests.

## fn require_record_or_table
The signature's `SyntaxShape` rejects a bad literal at parse time, but a dynamic
argument (`grimm dbg $x`) reaches the command's `run` unchecked - records are open and a list is
not shape-checked per element - so the guard has to live at run time too. One check
is genuinely not enough.

An empty list passes as a table, which matches the schema system's rule that `[]`
satisfies any table type.

## fn data_shape
Both members are column-open: an empty `CollectionColumns` declares no required
columns, so any record or table binds.

## fn event_id_record
One record for the three sends, so `grimm channel send` and the two remote
sends cannot drift from the `{event_id}` their documented examples give. A
`let` is where nushell enforces a declared output type, so a decl returning
other than it declares fails there, as a bare `Type mismatch`: a string under
a record signature did exactly that.

## fn event_id_type
The declared half of the same contract, shared by the three signatures.

## fn grimm_decls
One list behind both the registration and the signature block, so `interface help
grimm` can never document a decl an eval lacks, or miss one it has.

## fn grimm_signatures
`interface help grimm`'s block (User): a summary line, then the signature in
backticks, each decl in the list's order, the same two-line shape `interface help
interface` gives the interface's commands. Rendered from the decls themselves, so
the documentation is the signature the parser checks.

## fn signature_text
Nushell's own `--help` flag, which every signature carries, is left out: a grimm
command is never asked for its help that way.

## fn register_nuapi
Runs before `nu::parse`, as it must. The parser resolves command names at parse time, so a
decl added afterwards is invisible to the code that needed it.

The eval's registration is the access control. Every evaluating engine takes the
decls only inside `eval_in_process`, so they are absent from every engine that is
not mid-eval - not on the stateless base and not on the stateful one. Off-host
nushell cannot resolve the names at all. That is a stronger trap than a signing
scheme, because there is no reachable entry point to authenticate. A committed rig
can call them, but only because its call's body runs inside an eval.

The one other caller is `ParseEngine::new_full`, so the contract's parse of an `execute`
definition and the rig validator parse a `grimm` call as the eval will. That engine
parses and never evaluates, and a `grimm` decl is no const command, so the access
control stands. Without the decls a name the parser cannot resolve reads as an
external, and `grimm dbg [[x, y]; [1, 2]]` failed both parses on its table literal,
though the same text runs.

Every decl here binds nothing of a call: `dbg` and `send` find theirs at run
time, and the sends and the config trio ask the engine's controller through the
host link they were registered with (`system.rs`'s `HostLink`). They register
per eval rather than on the base,
because that is what keeps the whole family unreachable outside an eval; the
registration is the access control, not the routing. The parse engine's decls
take no link: it never evaluates.

## fn asked_host
A decl with no link is the parse engine's, which never runs one, so the refusal
is a guard rather than a path a body meets.

## fn unexpected
An answer its ask never takes is the controller's fault, refused in words that
name it rather than read as the body's.

## fn sendable
A value nuval can't carry is refused with its render's error, the words a send
gave before its event crossed as data.
