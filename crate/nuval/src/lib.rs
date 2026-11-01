pub(crate) mod access;
pub(crate) mod error;
pub(crate) mod kind {
    pub(crate) mod compile;
    pub(crate) mod config;
    pub(crate) mod io;
    pub(crate) mod job;
    pub(crate) mod network;
    pub(crate) mod parse;
    pub(crate) mod shell;
}
pub(crate) mod text {
    pub(crate) mod de;
    pub(crate) mod read;
    pub(crate) mod ser;
    pub(crate) mod write;
}
pub(crate) mod val;
pub(crate) mod val_error;
pub(crate) mod width;
#[cfg(all(test, feature = "nushell"))]
mod tests {
    mod access;
    mod text;
    mod val;
    mod val_error;
}

#[cfg(feature = "nushell")]
pub(crate) use crate::error::*;

#[cfg(feature = "nushell")]
pub(crate) use snafu::OptionExt;

pub(crate) use std::{fmt, ops::Range};

#[cfg(feature = "nushell")]
pub(crate) use std::ops::Bound;

pub(crate) use serde::{de, ser};

#[cfg(feature = "nushell")]
pub(crate) mod nu {
    pub(crate) use nu_protocol::{
        ChainedError, CompileError, ConfigError, ConfigWarning, ErrorLabel, Filesize, FloatRange,
        IntRange, LabeledError, ParseError, ParseWarning, Range, Record, ShellError, ShellWarning,
        Span, Value,
        ast::{CellPath, PathMember},
        casing::Casing,
        shell_error::{
            ErrorSite,
            io::ErrorKind as IoKind,
            job::JobError,
            network::{DnsErrorKind, NetworkError},
        },
    };
}

pub(crate) use sourcetrait_common::cereal;

pub use crate::{
    error::{NuonError, NuonResult},
    kind::{
        compile::CompileErrorKind,
        config::{ConfigErrorKind, ConfigWarningKind},
        io::{IoErrorKind, StdIoErrorKind},
        job::JobErrorKind,
        network::NetworkErrorKind,
        parse::{ParseErrorKind, ParseWarningKind},
        shell::{ShellErrorKind, ShellWarningKind},
    },
    text::{
        de::{from_nuon, from_nuval},
        ser::{to_nuon, to_nuval},
        write::NuonStyle,
    },
    val::{
        Bounded, CellPathMemberData, DateData, GlobData, NuVal, NuValResult, RangeData, SpanData,
        TypedRangeData,
    },
    val_error::{NuValError, NuValErrorKind, NuValErrorLabel, NuValErrorSource, NuValSeverity},
    width::wide,
};

#[cfg(feature = "nushell")]
pub use crate::{
    error::{ConvertError, ConvertResult},
    val::{TryFromNu, TryIntoNu},
};
