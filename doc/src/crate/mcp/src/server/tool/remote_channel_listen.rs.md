# remote_channel_listen.rs

## fn remote_listen
Blocks on the bind alone and answers the address bound, so an agent can hand a
random port to its peer; the pairing goes on behind, Connected or Disconnected
arriving on the Channel (`crate/peer`'s `link.rs`).

`--on` takes the entry's place for this listen alone; with neither, loopback on
a random port (`crate/peer`'s `remotes.rs`). A port taken refuses the listen,
and `--on` with the ip alone is the retry, so the bind never moves to another
port on its own.
