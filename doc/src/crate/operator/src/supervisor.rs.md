# supervisor.rs

## struct SupervisorConfig
The lines the operator starts with, the config's `[supervisor]` table resolved by its
host. Each is RELATIVE TO THE MACHINE - a fraction of the cores, of `MemTotal`, of a
filesystem's size, and the card's capacity less a headroom - so a default means the
same thing on another machine.

## fn check_fraction
The ONE range check, the file layer's and the pins' both, so a pin cannot reach a state
a config load would have refused.

Written to accept what lies INSIDE (0, 1] rather than to refuse what lies outside it:
NaN compares false against both bounds, so a check written the other way round lets it
through, and a gate reads every level as over a NaN line, warning at each re-warn floor
about nothing. A TOML float may be `nan`.
