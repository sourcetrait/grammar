# config.rs

## const CONFIG_FILE
The agent's config file under the workdir, the one the MCP reads from its
start directory, so a host's supervisor lines sit where its agent's do.

## const BASE_CONFIG
The MCP's embedded base, compiled in, whose `[supervisor]` lines are a worker's
defaults, so a worker warns at the levels an MCP does.

## struct SupervisorLines
The four `supervisor.*` lines, each optional, refusing an unknown one as the
MCP's file layer does.

## struct ConfigFile
The file as a worker reads it: its `[supervisor]` table alone, the MCP's
tables ignored, since a workdir's file may be the MCP's own.

## struct InstancePaths
### fn of
Every path is a function of the identity and the homes alone: the engine's
data and cache homes under the MCP's own `grammar/mcp/<id>/<namespace>` layout,
so a worker and the MCP share one namespace's files where both run on one
machine, and the namespace lock in the data home, the MCP's own.

## struct InstanceConfig
### fn load
Everything a start needs before any thread, plugin, or engine: the names
checked, the workdir found an absolute directory, the homes resolved, the paths
derived, and the supervisor lines read. A loaded config is a value: a later
edit of the file changes nothing it holds.

## fn read_supervisor
Each line the file's where it sets one, else the base's, each fraction held to
the operator's range. A missing file is the base alone; an unreadable or
malformed one is refused, never ignored.
