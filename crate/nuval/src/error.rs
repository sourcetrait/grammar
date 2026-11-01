use crate::*;

/// NUON text that didn't read, or a value that didn't convert to or from it.
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum NuonError {
    /// The text broke off, or held what NUON doesn't, at a byte offset.
    #[snafu(display("NUON at byte {offset}: {reason}"))]
    Read { offset: usize, reason: String },
    /// A value with no form in the other model, at its path.
    #[snafu(display("{}", if path.is_empty() { reason.clone() } else { format!("at {path}: {reason}") }))]
    Convert { path: String, reason: String },
}

impl NuonError {
    /// A conversion refused for `reason`, its path filled in as it unwinds.
    pub(crate) fn convert(reason: impl fmt::Display) -> Self {
        Self::Convert { path: String::new(), reason: reason.to_string() }
    }

    /// The error as met within `segment`, a record's field or a list's index.
    pub(crate) fn within(self, segment: impl fmt::Display) -> Self {
        match self {
            Self::Convert { path, reason } if path.is_empty() => {
                Self::Convert { path: segment.to_string(), reason }
            }
            Self::Convert { path, reason } => Self::Convert { path: format!("{segment}.{path}"), reason },
            read => read,
        }
    }
}

impl ser::Error for NuonError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::convert(msg)
    }
}

impl de::Error for NuonError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::convert(msg)
    }
}

/// A NUON read's or conversion's result.
pub type NuonResult<T> = Result<T, NuonError>;

/// A conversion between nushell and nuval that failed, and why.
#[cfg(feature = "nushell")]
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ConvertError {
    /// A value nuval doesn't carry: a closure or a custom value.
    #[snafu(display("a {value_type} value isn't serializable"))]
    Unsupported { value_type: String },
    /// A value that is itself an error, which only a result carries.
    #[snafu(display("an error value: {error}"))]
    ErrorValue { error: Box<nu::ShellError> },
    /// A record naming one column twice.
    #[snafu(display("the column `{column}` appears twice"))]
    DuplicateColumn { column: String },
    /// A range nushell would refuse to make.
    #[snafu(display("an invalid range: {reason}"))]
    Range { reason: String },
    /// A date outside chrono's range, or an offset beyond a day.
    #[snafu(display("an invalid date: {seconds}s and {seconds_fraction}ns at {utc_offset_minus}s"))]
    Date {
        seconds: i64,
        seconds_fraction: u32,
        utc_offset_minus: i32,
    },
    /// A cell path's row index past this platform's usize.
    #[snafu(display("the row index {value} is past this platform's usize"))]
    Index { value: u64 },
    /// A span past this platform's usize, or one ending before it starts.
    #[snafu(display("an invalid span: {start}..{end}"))]
    Span { start: u64, end: u64 },
}

/// A conversion's result.
#[cfg(feature = "nushell")]
pub type ConvertResult<T> = Result<T, ConvertError>;
