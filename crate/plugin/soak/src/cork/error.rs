use crate::*;

/// The result of a filesystem step.
pub(crate) type CorkResult<T> = Result<T, CorkError>;

/// A filesystem failure, with its path and the span its label takes.
#[derive(Debug, snafu::Snafu)]
pub(crate) enum CorkError {
    #[snafu(display("{op}: {path}", path = path.display()))]
    PathIO {
        source: io::Error,
        path: PathBuf,
        op: PathOp,
        span: Option<nu::Span>,
    },
    #[snafu(display("{kind} not found: {path}", path = path.display()))]
    PathNotFound {
        kind: PathKind,
        path: PathBuf,
        span: Option<nu::Span>,
    },
    #[snafu(display("Failed to copy {kind}: {from} >> {to}", from = from.display(), to = to.display()))]
    PathCopy {
        source: io::Error,
        kind: PathKind,
        from: PathBuf,
        to: PathBuf,
        span: Option<nu::Span>,
    },
    #[snafu(display("Expected UTF8: {s}", s = s.display()))]
    Utf8 {
        s: OsString,
        span: Option<nu::Span>,
    },
}

/// What a path names, for a message.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, snafu::Snafu)]
pub(crate) enum PathKind {
    #[default]
    #[snafu(display("Path"))]
    Path,
    #[snafu(display("Directory"))]
    Dir,
    #[snafu(display("File"))]
    File,
}

impl PathKind {
    /// nushell's not-found error kind for this kind of path.
    pub(crate) fn not_found_kind(&self) -> nu::IoErrorKind {
        match self {
            Self::Path | Self::File => nu::IoErrorKind::FileNotFound,
            Self::Dir => nu::IoErrorKind::DirectoryNotFound,
        }
    }
}

/// The filesystem operation that failed, for a message.
#[derive(Debug, Copy, Clone, PartialEq, Eq, snafu::Snafu)]
pub(crate) enum PathOp {
    #[snafu(display("Failed to create directory"))]
    CreateDir,
    #[snafu(display("Failed to read directory attributes"))]
    DirMeta,
    #[snafu(display("Failed to list directory"))]
    ListDir,
    #[snafu(display("Failed to write to file"))]
    WriteFile,
    #[snafu(display("Failed to read file"))]
    ReadFile,
}

impl ErrorTrait for CorkError {
    fn meta(&self) -> Option<ErrorMeta> {
        const UTF8: Option<ErrorMeta> = ErrorMeta::label("nu::cork::utf8", "non-utf8");

        match self {
            Self::PathIO { .. } => None,
            Self::PathNotFound { .. } => None,
            Self::PathCopy { .. } => None,
            Self::Utf8 { .. } => UTF8,
        }
    }

    fn into_span(self) -> Option<nu::Span> {
        match self {
            Self::PathNotFound { span, .. } => span,
            Self::PathCopy { span, .. } => span,
            Self::PathIO { span, .. } => span,
            Self::Utf8 { span, .. } => span,
        }
    }

    fn nu(self, span: nu::Span) -> nu::LabeledError {
        let msg = self.to_string();
        match self {
            Self::PathNotFound { path, kind, span: path_span } => {
                let error = nu::IoError::new(kind.not_found_kind(), path_span.unwrap_or(span), Some(path));
                io_labeled(error, msg)
            }
            Self::PathCopy { source, from, span: path_span, .. } => {
                let kind = nu::IoErrorKind::from_std(source.kind());
                let error = nu::IoError::new(kind, path_span.unwrap_or(span), Some(from));
                io_labeled(error, msg)
            }
            Self::PathIO { source, path, span: path_span, .. } => {
                let kind = nu::IoErrorKind::from_std(source.kind());
                let error = nu::IoError::new(kind, path_span.unwrap_or(span), Some(path));
                io_labeled(error, msg)
            }
            e @ Self::Utf8 { .. } => Self::default_nu(e, span),
        }
    }
}

/// nushell's labeled form of an I/O error, carrying `msg` and no help.
fn io_labeled(error: nu::IoError, msg: String) -> nu::LabeledError {
    let mut labeled = nu::LabeledError::from_diagnostic(&nu::ShellError::Io(error));
    labeled.msg = msg;
    labeled.help = None;
    labeled
}

impl CorkError {
    /// The labeled error of any error, its code and label from its meta.
    pub(crate) fn default_nu(e: impl ErrorTrait, span: nu::Span) -> nu::LabeledError {
        let msg = e.to_string();
        let meta = e.meta().unwrap_or_default();
        let span = e.into_span().unwrap_or(span);
        let mut e = nu::LabeledError::new(msg).with_code(meta.code);
        if let Some(label) = meta.label {
            e = e.with_label(label, span);
        }

        e
    }

    /// A failed operation on a path.
    pub(crate) fn path_io(
        source: io::Error,
        path: impl Into<PathBuf>,
        op: PathOp,
        span: impl Into<Option<nu::Span>>,
    ) -> Self {
        Self::PathIO { source, path: path.into(), op, span: span.into() }
    }

    /// A failed copy of a path.
    pub(crate) fn path_copy(
        source: io::Error,
        kind: PathKind,
        from: impl Into<PathBuf>,
        to: impl Into<PathBuf>,
        span: impl Into<Option<nu::Span>>,
    ) -> Self {
        Self::PathCopy { source, from: from.into(), to: to.into(), kind, span: span.into() }
    }

    /// A name that is not UTF-8.
    pub(crate) fn utf8(s: impl Into<OsString>, span: impl Into<Option<nu::Span>>) -> Self {
        Self::Utf8 { s: s.into(), span: span.into() }
    }

    /// A missing path, as an error result.
    pub(crate) fn err_path_not_found<T>(
        path: impl Into<PathBuf>,
        kind: PathKind,
        span: impl Into<Option<nu::Span>>,
    ) -> CorkResult<T> {
        Err(Self::PathNotFound { path: path.into(), kind, span: span.into() })
    }
}

/// An error's code and the label its span takes.
#[derive(Debug)]
pub(crate) struct ErrorMeta {
    pub(crate) code: &'static str,
    pub(crate) label: Option<&'static str>,
}

impl ErrorMeta {
    pub(crate) const DEFAULT_CODE: &str = "nu::shell::error";
    pub(crate) const DEFAULT_LABEL: &str = "here";
    pub(crate) const DEFAULT: Self = Self {
        code: Self::DEFAULT_CODE,
        label: Some(Self::DEFAULT_LABEL),
    };

    /// A code with a label.
    pub(crate) const fn label(code: &'static str, label: &'static str) -> Option<Self> {
        Some(Self { code, label: Some(label) })
    }

    /// A code alone.
    pub(crate) const fn code(code: &'static str) -> Option<Self> {
        Some(Self { code, label: None })
    }
}

impl Default for ErrorMeta {
    #[inline]
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// An error nushell can label: its meta, its span, and its labeled form.
pub(crate) trait ErrorTrait: Sized + Display {
    /// The code and label, for any error [Self::nu] does not build itself.
    fn meta(&self) -> Option<ErrorMeta>;

    /// The span the error's label takes, when it has its own.
    fn into_span(self) -> Option<nu::Span>;

    /// The error as nushell's labeled error, `span` its fallback.
    fn nu(self, span: nu::Span) -> nu::LabeledError {
        CorkError::default_nu(self, span)
    }
}
