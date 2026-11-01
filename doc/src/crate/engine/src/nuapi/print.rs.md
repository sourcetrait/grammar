# print.rs

## struct HostPrint
NOT nu-cli's `print`. Upstream's writes to the PROCESS's stderr in every protocol
mode (nu-cli `commands/print.rs`, nu-protocol `PipelineData::print_table`), and
in the host that stream is shared by every eval and read by nobody, the MCP
client's log at best. This one keeps upstream's signature, flags, and rendering
and writes to the running call's stderr log instead, the file an external's
stderr already lands in.

ONE DECL ON EVERY ENGINE, from `startup`, unlike the `grimm` family: `print` is
standard nushell, so the validator parses it as the eval runs it. It finds its
call at RUN time from `$env.NONCE` (`NuapiCall::current`), never at parse time:
a module an earlier stateful call parsed runs `print` in a later one, and a decl
carrying the earlier call's log wrote there - measured, the second call's output
landed in the first call's log.

The log is opened to append, as `eval_in_process` opens it, so a `print` and an
external's stderr interleave in the order written, neither overwriting the other.

`--stderr` is accepted and changes nothing, as under upstream's protocol modes,
where every `print` already goes to stderr.

## fn print_to
Upstream's two paths: `--raw` writes binary as bytes and everything else as its
plain text; otherwise a non-binary byte stream is copied as it arrives and the
rest goes through the `table` decl, as upstream's `print_table` renders it,
binary as a hexdump.
