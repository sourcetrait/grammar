//! grammard's command line: one session served on stdin and stdout.
use crate::*;

/// grammard's command line.
#[derive(clap::Parser)]
#[command(version, about = "The Grammar worker, an SSH session's command")]
pub(crate) struct GrammardCli {
    #[command(subcommand)]
    pub(crate) command: GrammardCmd,
}

/// grammard's commands.
#[derive(clap::Subcommand, Clone)]
pub(crate) enum GrammardCmd {
    /// Serve one session on stdin and stdout until it ends.
    Serve {
        /// The namespace's MCP id.
        #[arg(long)]
        id: String,
        /// The namespace.
        #[arg(long)]
        namespace: String,
        /// The working directory, an absolute path.
        #[arg(long)]
        workdir: PathBuf,
    },
}
