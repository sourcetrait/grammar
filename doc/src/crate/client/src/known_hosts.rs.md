# known_hosts.rs

An entry's known hosts as a file: OpenSSH's `known_hosts` format, read for one
host's keys as OpenSSH's client matches them, so a whole `~/.ssh/known_hosts`
named by an entry pins that host's keys and no other's.

## fn known_host_keys
Each line is split at its whitespace, tabs and runs of spaces included, and
parsed by ssh-key's known hosts entry, which russh re-exports, so a malformed
line is refused by its number rather than skipped. A `#` line and a blank one
are skipped. The host is matched lowercase, under `[host]:port` off port 22.
A `@cert-authority` line is skipped, no authority being configured; a
`@revoked` key matching the host is never pinned, whichever line names it
too. A key no session could verify, DSA's or an unknown algorithm's, is left
out. Each key is taken once, by its key data, as its line's own two fields, the
comment dropped. An empty answer is the caller's to refuse.

## fn host_matches
OpenSSH's `match_hostname`: any pattern of the list matching makes the line
the host's, and a negated one matching rules it out whatever else matches. A
hashed name is the HMAC-SHA1 of the name under its salt, checked by ring's
constant-time verify, ring being rustls's backend already.

## fn glob
OpenSSH's `match_pattern`, `*` any run of characters and `?` any one, with the
backtracking a `*` needs.
