# endpoint.rs

A remote engine as the MCP's config names it in `[[engine]]`: where its host
is, how the host is trusted, and what runs there. The MCP's config resolves
each entry, its defaults and its key's expansion, and holds it to `check`
before anything starts, so a bad entry fails the load rather than a call.

## const SSH_PORT
## const ENGINE_NAMESPACE
The port and the namespace an entry takes when it names none.

## struct Endpoint
The entry's fields, every one a string or a number, so the remote system
takes the entries as its parameters' data.

### fn check
The alias is a name by the protocol's rule and never `local`, which names the
MCP's own engine. The host and the user are one word each. The key file is
absolute once expanded; it is read at each connect, so a missing one fails a
call, not the load. At least one host key is pinned, each an OpenSSH public
key line of an algorithm whose signatures aren't SHA-1's: DSA's are, and a
certificate is refused, since no authority is configured to vouch for one.
The command and the workdir are absolute and of letters, digits, `/`, `.`,
`_`, `-`, and `+` alone, so the command line reaches any login shell, sh, zsh,
or nu, as it stands, with nothing to quote. The id and the namespace are an
instance's.

### fn command_line
The worker's command line, which sshd hands the account's login shell.

### fn instance
The instance the session's hello expects.

### fn pinned_keys
Each pinned line parsed, for the connection's handler.

## fn pinned_key
One pinned line: an OpenSSH public key, its comment ignored.

## fn shell_neutral_path
A path held absolute and to the shell-neutral characters.
