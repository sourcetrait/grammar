# config_channel.rs

## struct ConfigChannelResult
Returns what is in force whether or not this call changed it, so a caller never has to
assume its own update took.

## fn channel_config
Thresholds only. The port and the cert dir cannot change under a live hub, so they stay
set once at startup - offering them here would be a knob that silently does nothing.
