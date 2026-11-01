use crate::*;

/// The result of a soak.
pub(crate) type SoakResult<T> = Result<T, SoakError>;

/// A soak's failure.
#[derive(Debug, snafu::Snafu)]
pub(crate) enum SoakError {
    #[snafu(transparent)]
    Cork {
        source: CorkError,
    },
    #[snafu(display("Liquid parsing failed: {source}"))]
    LiquidParse {
        source: liquid::Error,
    },
    #[snafu(display("Cannot convert to Liquid value: {invalid}"))]
    LiquidValue {
        invalid: NuType,
        span: nu::Span,
    },
    #[snafu(display("Liquid template failed: {path}: {source}", path = path.display()))]
    Template {
        path: PathBuf,
        source: liquid::Error,
    },
    #[snafu(display(
        "Two sources write {output}: {first} and {second}",
        output = output.display(),
        first = first.display(),
        second = second.display(),
    ))]
    Collision {
        output: PathBuf,
        first: PathBuf,
        second: PathBuf,
    },
    #[snafu(display("The destination is inside the source: {dst}", dst = dst.display()))]
    DestinationInSource {
        dst: PathBuf,
        span: nu::Span,
    },
    #[snafu(display("An output would land inside the source: {output}", output = output.display()))]
    OutputInSource {
        output: PathBuf,
    },
    #[snafu(display("The destination is not a directory: {dst}", dst = dst.display()))]
    DestinationNotDirectory {
        dst: PathBuf,
        span: nu::Span,
    },
    #[snafu(display("Failed to walk the source: {source}"))]
    Walk {
        source: walkdir::Error,
    },
    #[snafu(display("Neither a file nor a directory: {path}", path = path.display()))]
    Unsupported {
        path: PathBuf,
    },
}

/// A nushell value's type Liquid cannot hold.
#[derive(Debug, snafu::Snafu)]
pub(crate) enum NuType {
    Binary,
    Custom,
    Closure,
    Date,
    Error,
}

impl ErrorTrait for SoakError {
    fn meta(&self) -> Option<ErrorMeta> {
        match self {
            Self::Cork { source } => source.meta(),
            Self::LiquidParse { .. } => ErrorMeta::code("nu::soak::liquid::parse"),
            Self::LiquidValue { invalid, .. } => {
                let label = match invalid {
                    NuType::Binary => "non-utf8",
                    NuType::Date => "non-standard",
                    NuType::Custom => "lacks base value",
                    _ => "incompatible",
                };

                ErrorMeta::label("nu::soak::value", label)
            },
            Self::Template { .. } => ErrorMeta::code("nu::soak::template"),
            Self::Collision { .. } => ErrorMeta::code("nu::soak::collision"),
            Self::DestinationInSource { .. } => ErrorMeta::label("nu::soak::overlap", "inside the source"),
            Self::OutputInSource { .. } => ErrorMeta::code("nu::soak::overlap"),
            Self::DestinationNotDirectory { .. } => {
                ErrorMeta::label("nu::soak::destination", "not a directory")
            },
            Self::Walk { .. } => ErrorMeta::code("nu::soak::walk"),
            Self::Unsupported { .. } => ErrorMeta::code("nu::soak::unsupported"),
        }
    }

    fn into_span(self) -> Option<nu::Span> {
        match self {
            Self::Cork { source } => source.into_span(),
            Self::LiquidValue { span, .. } => Some(span),
            Self::DestinationInSource { span, .. } => Some(span),
            Self::DestinationNotDirectory { span, .. } => Some(span),
            Self::LiquidParse { .. }
            | Self::Template { .. }
            | Self::Collision { .. }
            | Self::OutputInSource { .. }
            | Self::Walk { .. }
            | Self::Unsupported { .. } => None,
        }
    }

    fn nu(self, span: nu::Span) -> nu::LabeledError {
        match self {
            Self::Cork { source } => source.nu(span),
            other => CorkError::default_nu(other, span),
        }
    }
}
