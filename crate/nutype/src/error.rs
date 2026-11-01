use crate::*;

/// Type text that didn't read, a shape nutype doesn't hold, or a value that
/// isn't of its type.
#[derive(Debug, PartialEq, Eq, snafu::Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum NuTypeError {
    /// Type text that broke off or named no type, at a byte offset.
    #[snafu(display("type text at byte {offset}: {reason}"))]
    Read { offset: usize, reason: String },
    /// A nushell shape or type no value nuval holds can take.
    #[snafu(display("`{shape}` isn't a data type"))]
    Unsupported { shape: String },
    /// A value that isn't of its type, at its path.
    #[snafu(display("{}", if path.is_empty() { reason.clone() } else { format!("at {path}: {reason}") }))]
    Mismatch { path: String, reason: String },
}

impl NuTypeError {
    /// The mismatch as met within `segment`, a record's field or a list's
    /// index.
    pub(crate) fn within(self, segment: impl fmt::Display) -> Self {
        match self {
            Self::Mismatch { path, reason } if path.is_empty() => {
                Self::Mismatch { path: segment.to_string(), reason }
            }
            Self::Mismatch { path, reason } => Self::Mismatch { path: format!("{segment}.{path}"), reason },
            other => other,
        }
    }
}

/// A type's read, conversion, or check result.
pub type NuTypeResult<T> = Result<T, NuTypeError>;
