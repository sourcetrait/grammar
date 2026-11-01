# lib.rs

The remote worker as a library, and grammar-ssh over it. A worker is an SSH
session's command: the MCP opens the session and runs `grammar-ssh serve` as its
command, the session's stdio the wire, and the worker's work ends with the
session, as a command's does when its SSH session ends (User). The library
builds without the `service` feature, so an embedding host links the runtime
without the command line; grammar-ssh is the binary the feature adds.

## mod cli, mod run
grammar-ssh's command line and entry, behind `service`.

## mod controller
The worker's systems, the engine's and the operator's, under the one
controller its process owns, and the way their asks and emergencies reach the
session's MCP.

## mod instance
The namespace a worker serves: its names, its homes, its paths, its workdir,
and its supervisor lines, resolved before anything runs.

## mod runtime
The worker's start, its lock and its scratch, and its shutdown.

## mod session
The session on stdin and stdout: the MCP's hello answered, its requests served,
and its end.

## pub use
The runtime and its identity, the worker's scratch, the instance's types, the
session's end, the error and its kind, and, with `service`, grammar-ssh's entry.
