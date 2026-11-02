//! grammar-ssh's command line; a bare invocation serves on stdin and stdout.
use crate::*;

/// grammar-ssh's command line: the namespace it serves, and where.
#[derive(clap::Parser)]
#[command(version, about = "Grammar SSH")]
pub(crate) struct Cli {
    /// Agent identity owning the state namespace; defaults to the invoking user.
    #[arg(long, default_value_t = default_id())]
    pub(crate) id: String,
    /// State namespace within the id; defaults to `default`.
    #[arg(long, default_value = DEFAULT_NAMESPACE)]
    pub(crate) namespace: String,
    /// Agent working directory, absolute; the session's start directory by default.
    #[arg(long)]
    pub(crate) workdir: Option<PathBuf>,
}

/// The invoking user, the id the MCP takes when none is named.
fn default_id() -> String {
    std::env::var("USER").unwrap_or_else(|_| "default".to_string())
}
