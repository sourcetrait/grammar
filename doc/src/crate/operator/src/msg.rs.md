# msg.rs

The operator's protocol with its manager. Both directions are SINGULAR packets, no
request waiting on an answer: an observation is the manager's news, and an emergency
the operator's. The data they carry is the protocol crate's, portable and shared
without any engine dependency.

## type ThreadLane
The engine's own lane under the operator's name, so the operator's callers compile as
they did; a watched thread's lane needs no mapping.

## type EngineReading
The engine's health as the supervisor reads it, the engine's own `EngineHealth`: the
watched threads, the permits held against the cap, and the jobs table's size. The
manager passes the engine's answer through unchanged.
