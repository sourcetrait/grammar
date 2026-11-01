# start.rs

## static STARTED
One runtime a process, and only once: the engine underneath holds process-wide
state that initializes once and never again, so a second runtime, under
another identity or the same one after a shutdown, can't own an engine of its
own. The claim is an atomic swap, so two starts racing on two threads leave
exactly one runtime. A start that fails before the engine starts gives the
claim back, so the process may try again; once the engine has started the
claim stays taken.

## struct RuntimeIdentity
The worker's identity is the effective UID with the MCP id and the namespace:
the UID is the account the SSH session logged in as, and same-UID namespaces
separate state alone. The identity is fixed at the start and never changes for
the runtime's life.

## struct Runtime
What grammard or an embedding host owns: started explicitly, a session served
on it, and shut down explicitly.

### fn start
The order is the namespace's safety. The lock comes before anything that
touches the namespace, so a second worker, or an MCP on the namespace, is
refused before its scratch, its engine, or a plugin; then the holder is
recorded in the lock, the scratch made, the task runtime made, the engine's
parameters, config dirs, and substrate set, once, and the controller's systems
started on it. A failure undoes what the start made, the tasks stopped, the
scratch removed, and the lock released with its descriptor, and leaves the
namespace's state and equips as they were; one before the engine gives the
claim back. The worker's nom is drawn at each start, before the lock, so the
record names the nom the engine exports.

### fn profile
The worker's identity and bounds as its hello's reply names them, its limits
the protocol's defaults.

### fn info
What `interface info` reports of the host that only the worker knows.

### fn serve
The session runs on the worker's tasks, its own start done, so its hello is
answered by a worker that serves.

### fn shutdown
The systems stop first, the tasks are given a short grace, the scratch goes,
and the lock last, so a successor takes the namespace only once this worker is
done with it. The process's claim stays taken, matching the engine's single
initialization.

## fn tasks
The worker's own threads, sized for the parses run on them as the MCP's are,
an eval thread's stack each: an interface parse or a definition's check runs
inline on a runtime worker.

## fn claim
The scratch crate's namespace lock (`crate/scratch`'s `lock.rs`), the one the
MCP takes too, recorded with the worker's nom.

## fn start_engine
The MCP's substrate, made as its host makes it: the equip store, the default
purview, the plugin registry, and the builtins on a fresh store. A default
purview that can't be made is reported, as the MCP's host reports it, and the
start goes on.
