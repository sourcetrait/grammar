# cli.rs

## struct GrammarSshCli
## enum GrammarSshCmd
One command, `serve`, its id and namespace named on the command line the MCP's
entry configures, and its workdir where the entry names one, so a worker reads
no file to know what it serves: the MCP's config is the one place a remote host
is described. A workdir left out is the directory the session starts in, the
account's home under sshd. A missing argument is clap's usage error, exit 2,
before anything starts.
