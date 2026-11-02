# config.rs

## const CONFIG_FILE
The agent's config file under the workdir, the one the MCP reads from its
start directory, so grammar-ssh's supervisor lines sit where its agent's do.

## const BASE_CONFIG
The MCP's embedded base, compiled in, whose `[supervisor]` lines are
grammar-ssh's defaults, so it warns at the levels the MCP does.

## const NAMESPACES
The namespace's directories sit under the MCP's own `grammar/mcp/<id>/<namespace>`
in each home, so grammar-ssh and the MCP share one namespace's files where both
run on one machine, the namespace lock among them.

## struct SupervisorLines
The four `supervisor.*` lines, each optional, refusing an unknown one as the
MCP's file layer does.

## struct ConfigFile
The file as grammar-ssh reads it: its `[supervisor]` table alone, the MCP's
tables ignored, since a workdir's file may be the MCP's own.

## struct Name
One form for an id and a namespace alike, the protocol's `valid_name`
(`crate/protocol`'s `hello.rs`), so grammar-ssh and the MCP refuse the same
names. A letter or a digit first rules out `.`, `..`, and every hidden or
option-like name, and the rest holds no separator, NUL, space, or byte past
ASCII, so a name is always one plain path component. A name outside the form
is refused with the field it was given as; nothing is sanitized, since a
sanitized name could become another namespace's.

## struct Config
### fn load
Everything a start needs before any thread, plugin, or engine: the names
checked, the workdir found an absolute directory, the homes resolved, and the
supervisor lines read. A loaded config is a value: a later edit of the file
changes nothing it holds.

### fn data_home, fn cache_home, fn lock_file
The MCP's `data_home` and `cache_home` and its namespace lock, each a function
of the namespace and the homes alone. The cache home is what grammar-ssh's end
removes (`controller.rs`).

## fn read_supervisor
Each line the file's where it sets one, else the base's, each fraction held to
the operator's range. A missing file is the base alone; an unreadable or
malformed one is refused, never ignored.
