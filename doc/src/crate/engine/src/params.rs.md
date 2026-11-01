# params.rs

## struct EngineParams
What the engine reads of its host, given once at its start rather than read from
the host's globals: the six EQUIP values a definition sees, the two scratch
directories and the host process's nom among them, and the namespace's two homes, the data home holding
the lib dirs and the plugin registry, the cache home holding the call logs and
the cached definitions. The host builds them from its own config, the scratch
directories made before them.

## fn params
`None` before `init`, so the registry's path reads as no registry rather than a
panic: an engine nothing started carries no plugins.

## fn started
Every other reader is past `init` by construction: the host starts the engine
before it builds any, so a missing value is a startup-order bug, as an early read
of the host's own config is.

## fn init
Once a process: the parameters, then nushell's config dirs, which need the
registry's path in the data home. A second start is refused. A failed
resolution leaves the parameters set, since the host exits on it. The host is
no parameter: each engine system's controller answers its evals' asks
(`system.rs`), so one process may run several, each on its own controller.
