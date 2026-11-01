# cert.rs

## fn ensure_cert_profile

Ship-and-continue, deliberately not the library's shut-down-if-absent pattern.
srcert's README prescribes a consumer that ships its profile then shuts down
until the operator runs the tool - right for a service whose TLS identity is
mandatory. grammar-mcp's channel is optional and lazy: a session may never open
one, and the host does far more than the channel, so stopping it over a missing
optional-feature cert would be wrong. So this ships the profile when absent,
logs the operator hint once (on the first ship), and returns; an identity the
config file does not name, or one whose files are missing, surfaces later as
the channel's own `ChannelStart` error at open. Both servers ship it as they
start, the client and the test driver, since the Channel and the remote
channels are each's own.

The hint names both steps, since the generated pair does nothing until the
config file names it: srcert writes the certificate and key, and the operator
names them as the file's `[identity]`.

Every path is a stderr line, never a return value or a stop - grammar-mcp's fd 1
is the JSON-RPC channel, so operator messages ride stderr like every other host
log line. The profile text is embedded (`include_str!`), so changing it needs a
rebuild, as with the config defaults.
