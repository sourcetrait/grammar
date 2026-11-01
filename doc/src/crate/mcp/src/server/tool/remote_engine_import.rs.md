# remote_engine_import.rs

`interface remote engine import`, an entry's host fields from an ssh
configuration's host, a command of its own beside `set` (User).

## fn NuSh::engine_import
The alias is checked before ssh runs. `ssh -G` and `ssh-keygen` block, so they
run on a blocking thread, never on the runtime's workers. The fields read are
written as a set writes them, the entry's other fields left as they stand.
