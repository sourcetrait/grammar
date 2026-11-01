# remote_channel_connect.rs

## fn remote_connect
`--address` takes the entry's place for this connect alone, as a listener's
random port must be handed to it; with neither, the connect is refused before
any dial. The dial runs on a task of the peer links' own, so their system
serves on while it waits.
