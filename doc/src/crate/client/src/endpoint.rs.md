# endpoint.rs

A remote engine as the remotes file's `[[engine]]` entry names it: where its
host is, how the host is trusted, and what runs there. The MCP reads the entry
at each use, so a change takes effect at the next session the entry opens.
The field checks hold each field a command gives before the file is written,
and `check` holds the whole entry before a session opens on it.

## const SSH_PORT
## const ENGINE_NAMESPACE
The port and the namespace an entry takes when it names none.

## struct Endpoint
The entry's fields, every one a string or a number, so the remote system
takes an endpoint as a request's data.

### fn check
Each field held to its check, the refusal naming the field.

### fn command_line
The worker's command line, which sshd hands the account's login shell.

### fn instance
The instance the session's hello expects.

### fn pinned_keys
Each pinned line parsed, for the connection's handler.

## fn check_alias
The alias is a name by the protocol's rule and never `local`, which names the
MCP's own engine.

## fn check_word
The host and the user are one word each.

## fn check_port
Port zero is the kernel's pick, never a server's.

## fn check_key
The key file is absolute once expanded. It is read at each connect, so a
missing one fails the connect, not the entry.

## fn check_host_keys
At least one host key is pinned, each an OpenSSH public key line of an
algorithm whose signatures aren't SHA-1's: DSA's are, and a certificate is
refused, since no authority is configured to vouch for one.

## fn check_path
The command and the workdir are absolute and of letters, digits, `/`, `.`,
`_`, `-`, and `+` alone, so the command line reaches any login shell, sh, zsh,
or nu, as it stands, with nothing to quote.

## fn check_instance
The id and the namespace are an instance's.

## fn pinned_key
One pinned line: an OpenSSH public key, its comment ignored.

## fn shell_neutral_path
A path held absolute and to the shell-neutral characters.
