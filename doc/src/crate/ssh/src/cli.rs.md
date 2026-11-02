# cli.rs

## struct Cli
A bare invocation serves, as the MCP's does, its flags the MCP's own: `--id`,
the invoking user's by default, `--namespace`, `default` by default, and
`--workdir`. The MCP's remote engine entry names the three on the command line
it runs, so grammar-ssh reads no file to know what it serves; a workdir left
out is the directory the session starts in, the account's home under sshd, as
the entry's own documentation promises. An argument it takes none of is clap's
usage error, exit 2, before anything starts.

## fn default_id
`USER`, else `default`, the MCP's own default.
