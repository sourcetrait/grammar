//! grammar-ssh's command line: one session served on stdin and stdout.
use crate::*;

/// grammar-ssh's command line.
#[derive(clap::Parser)]
#[command(version, about = "The Grammar worker, an SSH session's command")]
pub(crate) struct GrammarSshCli {
    #[command(subcommand)]
    pub(crate) command: GrammarSshCmd,
}

/// grammar-ssh's commands.
#[derive(clap::Subcommand, Clone)]
pub(crate) enum GrammarSshCmd {
    /// Serve one session on stdin and stdout until it ends.
    Serve {
        /// The namespace's MCP id.
        #[arg(long)]
        id: String,
        /// The namespace.
        #[arg(long)]
        namespace: String,
        /// The working directory, an absolute path; where the session starts by default.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}
