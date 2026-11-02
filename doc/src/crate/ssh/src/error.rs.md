# error.rs

## enum GrammarSshError
Plain data with a hand-written `Display`, so grammar-ssh prints the same text it
exits by and refuses an MCP's hello with. The config's refusals, a name, a
workdir, a config file, or a home, each carry what they refused, so a report
names the very name, path, or home at fault. The MCP reports its own start's
failures as text on its stderr (the MCP's `run.rs`); grammar-ssh's reach the MCP
in its hello's refusal, so each carries a kind.

## fn GrammarSshErrorKind::name
The kind as a refusal or a test names it, by subject as the MCP's are: the
config's under `config::`, the scratch's under `scratch::`, the cache home's
under `cache::`, the namespace lock's under `namespace::`, the engine's start
under `engine::`, the host's own threads under `host::`, and a session that
couldn't open under `session::`.

## fn exit_code
sysexits' meanings, so a supervisor or a test tells the failures apart without
reading text: 64, `EX_USAGE`, for a name that is not one; 78, `EX_CONFIG`, for a
workdir, a config file, or a home the start refuses; 73, `EX_CANTCREAT`, for
scratch or a cache home grammar-ssh can't make or remove and a lock it can't
take for a reason of the lock's own; 75, `EX_TEMPFAIL`, for a namespace another
process holds, which a later start may find free; 71, `EX_OSERR`, for task
threads the system won't give; 76, `EX_PROTOCOL`, for a session whose hello was
refused or whose stream broke before it; and 70, `EX_SOFTWARE`, for an engine
that fails its start.

## impl From<LockError>
The scratch crate's lock refusals as grammar-ssh's own kinds, the holder's
record kept, so a held namespace reports `namespace::held` naming its holder.
