use crate::*;

/// Any displayable error as a labeled error at the call's head.
pub(crate) fn labeled_error(message: impl Display, head: nu::Span) -> nu::LabeledError {
    let message = message.to_string();
    nu::LabeledError::new(message.clone()).with_label(message, head)
}

/// The result of rendering a tree.
pub(crate) type TreeResult<T> = Result<T, TreeError>;

/// A tree's failure: a path unreadable, a glob invalid, or a walk failed.
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum TreeError {
    #[snafu(display("could not read: {}", path.display()))]
    Read {
        path: PathBuf,
        source: io::Error,
    },

    #[snafu(display("invalid glob pattern: {pattern}"))]
    Glob {
        pattern: String,
        source: nu_glob::PatternError,
    },

    #[snafu(display("could not walk {}: {source}", path.display()))]
    Walk {
        path: PathBuf,
        source: ignore::Error,
    },
}
