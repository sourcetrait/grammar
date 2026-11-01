# lib.rs

The guard over owned scratch directories and the namespace lock, shared by
every process that keeps them: the MCP and the worker. It holds neither the
engine nor the operator, so any product links it without either.

## mod dir, liveness, owner, sweep
The owned directory, its owner's liveness, the marker, and the sweep.

## mod lock
The namespace lock and its holder's record.
