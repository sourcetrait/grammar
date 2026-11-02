# endpoint.rs

A remote engine as the remotes file's `[[engine]]` entry resolves it: where its
host is, how the host is trusted, and what runs there. The MCP reads the entry
at each use, so a change takes effect at the next session the entry opens.
The field checks hold each field a command gives before the file is written,
and `check` holds the whole entry before a session opens on it.

## const SSH_PORT
The port an address takes when it names none.

## const GRAMMAR_SSH_COMMAND
grammar-ssh's command when an entry names none: a bare name, found on the
`PATH` of the account's login, where a complete install links each of
grammar's binaries from the user's local bin.

## enum Platform
The host's operating system as its entry names it, `windows`, `linux`, or
`macos`, never a family (User): the code groups them where a choice needs it,
Linux and macOS alike in `is_unix`. It decides the exec command and the path
rules and nothing else. An entry gives it with no default, so no host takes
another's exec command by an omission.

## struct Jump
The host a session reaches its engine's host through, as another entry names it:
that entry's host, port, user, key, and pinned keys, taken from its own resolved
endpoint, so the jump is trusted and logged in by its own fields. A jump runs no
grammar-ssh, so its command, its id, and its workdir play no part.

### fn place
`the jump `<alias>` at <host>:<port>`, which every refusal on the jump's hop
carries, so a failure says the jump failed, not the engine's own host.

## struct Endpoint
The entry's fields resolved, the address split into its host and port, the
known hosts read into the pinned keys, and the defaults taken, every field a
string, a number, or an optional string, so the remote system takes an
endpoint as a request's data.

### fn check
Each field held to its check, the refusal naming the field. A jump naming the
entry itself is refused, and a jump's own fields are held to a login's checks
under the jump's alias.

### fn place
The host's address, and where a jump is taken `<host>:<port> through the jump
`<alias>``, which every refusal on the engine's own hop carries then; a
direct session's refusals read as they did before jumps.

### fn command_line
The exec command sshd hands the account's shell: `exec /bin/sh -c 'exec
"$SHELL" -l -c "exec <grammar-ssh line>"'`. Grammar runs as the user, logged in,
so grammar-ssh starts in its account's login, as the MCP runs in its user's:
sshd runs a command through the account's shell neither interactive nor a
login, so a profile's `PATH` and variables reach it only where that shell
reads them unasked, bash's `~/.bashrc` on Fedora, zsh's `~/.zshenv`, nu
nothing at all. The outer line reads alike in sh, bash, zsh, fish, and nu;
`/bin/sh` expands `SHELL`, which sshd sets from the account's entry, and its
`-l` runs the profile before the last `exec` replaces the shell with
grammar-ssh, so it holds the channel's stdin and stdout itself, a bare
command resolves on the login's `PATH`, and every call's externals see the
login's environment. A shell taking no `-l -c`, csh's or tcsh's, can't run
grammar-ssh.

A `windows` host's line goes bare. Windows has no `/bin/sh`, and its sshd
hands the line to the shell `DefaultShell` names, PowerShell, cmd, or nu, each
of which runs `<command> <args>` as it stands, a bare command found by the
shell's own lookup on the login's `Path`. Windows' sshd builds a session's
environment from the account's registry, so grammar-ssh sees the account's
variables with no login shell between.

### fn grammar_ssh_line
grammar-ssh's own line, its workdir given where the entry names one, every
word of the shell-neutral characters the checks allow, so none of the three
shells it passes through reads anything into it.

### fn instance
The instance the session's hello expects.

### fn pinned_keys
Each pinned line parsed, for the connection's handler.

## fn check_alias
An alias is names joined by `/`, each a name by the protocol's rule, so `ai/lab`
nests, and none is `local`, which names the MCP's own engine. An import's
known hosts file sits at the alias's path, so the rule keeps every segment a
plain path component: no empty one, no `.` or `..`.

## fn parse_address
`<host>[:<port>]`, a channel's address shape with a host name allowed, port 22
by default. An IPv6 host goes in brackets, with or without a port, as
OpenSSH's known hosts write it; a bare one is refused, its colons ambiguous.

## fn known_hosts_name
The name a known hosts file lists a host under, OpenSSH's own:
`[host]:port` off port 22.

## fn check_login
The fields every SSH login takes, the engine's own and its jump's alike, in
the order a refusal reports them.

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

## fn check_command
The command is an absolute path on the entry's platform or a bare name, the
protocol's name rule, so it never names a path relative to wherever the login
starts.

## fn check_path
The workdir is absolute on the entry's platform; see `shell_neutral_path`.

## fn is_absolute
Unix's absolute path starts at `/`, Windows' at a drive, `C:\`: a rooted
`\path` or a drive-relative `C:path` resolves against the session's current
drive or directory, so neither passes.

## fn check_instance
The id and the namespace are an instance's.

## fn pinned_key
One pinned line: an OpenSSH public key, its comment ignored.

## fn shell_neutral_path
A Unix path held absolute and to letters, digits, `/`, `.`, `_`, `-`, and `+`
alone, so the command line reaches any login shell, sh, zsh, or nu, as it
stands, with nothing to quote, through the wrapper's three layers.

A Windows path is held, past its drive's `C:`, to letters, digits, `\`, `.`,
`_`, and `-` alone. Its line goes bare to PowerShell, cmd, or nu: a space parts
it in each, and `$`, `%`, `^`, `&`, `;`, `+`, and the quotes each mean something
to one of them.
