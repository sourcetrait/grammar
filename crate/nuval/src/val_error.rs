use crate::*;

/// A nushell error or warning as data: its kind, message, labels, and help.
#[cereal::derived(Data)]
pub struct NuValError {
    pub kind: NuValErrorKind,
    pub severity: NuValSeverity,
    pub msg: String,
    pub code: Option<String>,
    pub labels: Vec<NuValErrorLabel>,
    pub help: Option<String>,
    pub url: Option<String>,
    pub cause: Option<String>,
    pub source: Option<NuValErrorSource>,
    pub inner: Vec<NuValError>,
}

/// Which built-in nushell error an error is, by the type and variant it was.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub enum NuValErrorKind {
    Shell(ShellErrorKind),
    Parse(ParseErrorKind),
    Compile(CompileErrorKind),
    Config(ConfigErrorKind),
    Io(IoErrorKind),
    Network(NetworkErrorKind),
    Job(JobErrorKind),
    ShellWarning(ShellWarningKind),
    ParseWarning(ParseWarningKind),
    ConfigWarning(ConfigWarningKind),
    Generic,
    Labeled,
    Chained,
    Other,
}

impl NuValErrorKind {
    /// The diagnostic code nushell gives the kind, where it gives one.
    pub const fn code(self) -> Option<&'static str> {
        match self {
            Self::Shell(k) => k.code(),
            Self::Parse(k) => k.code(),
            Self::Compile(k) => k.code(),
            Self::Config(k) => k.code(),
            Self::Io(k) => k.code(),
            Self::Network(k) => k.code(),
            Self::Job(k) => k.code(),
            Self::ShellWarning(k) => k.code(),
            Self::ParseWarning(k) => k.code(),
            Self::ConfigWarning(k) => k.code(),
            Self::Chained => Some(CHAINED_CODE),
            Self::Generic | Self::Labeled | Self::Other => None,
        }
    }

    /// The kind's message, the same for every error of the kind.
    pub const fn message(self) -> &'static str {
        match self {
            Self::Shell(k) => k.message(),
            Self::Parse(k) => k.message(),
            Self::Compile(k) => k.message(),
            Self::Config(k) => k.message(),
            Self::Io(k) => k.message(),
            Self::Network(k) => k.message(),
            Self::Job(k) => k.message(),
            Self::ShellWarning(k) => k.message(),
            Self::ParseWarning(k) => k.message(),
            Self::ConfigWarning(k) => k.message(),
            Self::Generic => "A generic error",
            Self::Labeled => "A labeled error",
            Self::Chained => "An error raised through others",
            Self::Other => "An error from outside nushell",
        }
    }
}

/// The code nushell gives an error chained through others.
const CHAINED_CODE: &str = "chained_error";

/// How severe a diagnostic is.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub enum NuValSeverity {
    Error,
    Warning,
    Advice,
}

/// A span an error points at, with its label's text.
#[cereal::derived(Data)]
pub struct NuValErrorLabel {
    pub text: Option<String>,
    pub span: SpanData,
    pub primary: bool,
}

/// Source text an error carries itself, which its labels' spans index.
#[cereal::derived(Data)]
pub struct NuValErrorSource {
    pub name: Option<String>,
    pub text: String,
}

#[cfg(feature = "nushell")]
impl From<miette::LabeledSpan> for NuValErrorLabel {
    fn from(v: miette::LabeledSpan) -> Self {
        let start = wide(v.offset());
        Self {
            text: v.label().map(String::from),
            span: SpanData {
                start,
                end: start.saturating_add(wide(v.len())),
            },
            primary: v.primary(),
        }
    }
}

impl NuValError {
    /// An error from a diagnostic, its kind, carried source, and inner given.
    #[cfg(feature = "nushell")]
    fn of(
        kind: NuValErrorKind,
        diag: &dyn miette::Diagnostic,
        source: Option<NuValErrorSource>,
        inner: Vec<NuValError>,
    ) -> Self {
        let cause = diag
            .diagnostic_source()
            .map(|d| d.to_string())
            .or_else(|| diag.source().map(|e| e.to_string()));
        Self {
            kind,
            severity: match diag.severity() {
                Some(miette::Severity::Warning) => NuValSeverity::Warning,
                Some(miette::Severity::Advice) => NuValSeverity::Advice,
                Some(miette::Severity::Error) | None => NuValSeverity::Error,
            },
            msg: diag.to_string(),
            code: diag.code().map(|c| c.to_string()),
            labels: diag
                .labels()
                .into_iter()
                .flatten()
                .map(NuValErrorLabel::from)
                .collect(),
            help: diag.help().map(|h| h.to_string()),
            url: diag.url().map(|u| u.to_string()),
            cause,
            source,
            inner,
        }
    }

    /// The error on one line: its message, labels, cause, help, and inner.
    pub fn short(&self) -> String {
        let msg = one_line(&self.msg);
        let labels: Vec<String> = self
            .labels
            .iter()
            .filter_map(|label| label.text.as_deref())
            .map(one_line)
            .filter(|text| !text.is_empty() && !repeats(&msg, text))
            .collect();
        let mut out = if labels.is_empty() {
            msg
        } else {
            let head = msg.strip_suffix('.').unwrap_or(&msg);
            format!("{head}: {}", labels.join(", "))
        };
        if let Some(cause) = self.cause.as_deref().map(one_line).filter(|c| !c.is_empty()) {
            out.push_str(" - ");
            out.push_str(&cause);
        }
        if let Some(help) = self.help.as_deref().map(one_line).filter(|h| !h.is_empty()) {
            out.push_str(" (");
            out.push_str(&help);
            out.push(')');
        }
        for inner in &self.inner {
            out.push_str("; ");
            out.push_str(&inner.short());
        }
        out
    }

    /// An error from a diagnostic outside nushell's own types.
    #[cfg(feature = "nushell")]
    pub fn from_diagnostic(diag: &dyn miette::Diagnostic) -> Self {
        let inner = diag
            .related()
            .into_iter()
            .flatten()
            .map(Self::from_diagnostic)
            .collect();
        Self::of(NuValErrorKind::Other, diag, None, inner)
    }
}

/// Text on one line, each run of whitespace one space, trimmed.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whether a message already ends with a label's text, case and a period aside.
fn repeats(
    msg: &str,
    label: &str,
) -> bool {
    let fold = |text: &str| text.trim_end_matches('.').to_lowercase();
    fold(msg).ends_with(&fold(label))
}

/// Shell errors as data, in order.
#[cfg(feature = "nushell")]
fn shells(errors: &[nu::ShellError]) -> Vec<NuValError> {
    errors.iter().map(NuValError::from).collect()
}

/// The source text a shell error carries itself, its labels' spans into it.
#[cfg(feature = "nushell")]
fn shell_source(e: &nu::ShellError) -> Option<NuValErrorSource> {
    match e {
        nu::ShellError::OutsideSpannedLabeledError { src, .. } => Some(NuValErrorSource {
            name: None,
            text: src.clone(),
        }),
        nu::ShellError::OutsideSource { src, .. }
        | nu::ShellError::OutsideSourceNoUrl { src, .. } => Some(NuValErrorSource {
            name: Some(src.name().to_string()).filter(|n| !n.is_empty()),
            text: src.inner().clone(),
        }),
        _ => None,
    }
}

/// A shell error's related errors, each converted by its own type.
#[cfg(feature = "nushell")]
#[allow(deprecated)]
fn shell_inner(e: &nu::ShellError) -> Vec<NuValError> {
    match e {
        nu::ShellError::InvalidConfig { errors } => errors.iter().map(NuValError::from).collect(),
        nu::ShellError::GenericError { inner, .. }
        | nu::ShellError::OutsideSource { inner, .. }
        | nu::ShellError::OutsideSourceNoUrl { inner, .. } => shells(inner),
        nu::ShellError::EvalBlockWithInput { sources, .. } => shells(sources),
        _ => Vec::new(),
    }
}

/// The location text an error raised inside nushell carries as its source.
#[cfg(feature = "nushell")]
fn location_source(location: &str) -> NuValErrorSource {
    NuValErrorSource {
        name: None,
        text: location.to_string(),
    }
}

/// A chain's first error is the error itself; a later chain wraps its errors.
#[cfg(feature = "nushell")]
fn chained(c: &nu::ChainedError) -> NuValError {
    let sources: Vec<nu::ShellError> = c.clone().sources_iter().collect();
    match sources.first() {
        Some(first) if chain_is_first(c) => NuValError::from(first),
        _ => NuValError::of(NuValErrorKind::Chained, c, None, shells(&sources)),
    }
}

/// Whether a chain is its first link, which nushell keeps in a private field.
#[cfg(feature = "nushell")]
fn chain_is_first(c: &nu::ChainedError) -> bool {
    const FIRST: &str = "ChainedError { first: true";
    let mut prefix = Prefix(String::with_capacity(FIRST.len()), FIRST.len());
    let _ = std::fmt::write(&mut prefix, format_args!("{c:?}"));
    prefix.0 == FIRST
}

/// A writer keeping the first bytes written and refusing the rest.
#[cfg(feature = "nushell")]
struct Prefix(String, usize);

#[cfg(feature = "nushell")]
impl std::fmt::Write for Prefix {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let room = self.1 - self.0.len();
        let take = s.char_indices().map(|(i, _)| i).nth(room).unwrap_or(s.len());
        self.0.push_str(&s[..take]);
        if self.0.len() < self.1 {
            Ok(())
        } else {
            Err(std::fmt::Error)
        }
    }
}

#[cfg(feature = "nushell")]
impl From<&nu::ShellError> for NuValError {
    fn from(e: &nu::ShellError) -> Self {
        if let Some(kind) = ShellErrorKind::of(e) {
            return Self::of(NuValErrorKind::Shell(kind), e, shell_source(e), shell_inner(e));
        }
        match e {
            nu::ShellError::Network(network) => {
                Self::of(NuValErrorKind::Network(network.into()), e, None, Vec::new())
            }
            nu::ShellError::Io(io) => {
                let source = io
                    .location
                    .as_deref()
                    .filter(|_| io.span == nu::Span::unknown())
                    .map(location_source);
                Self::of(NuValErrorKind::Io((&io.kind).into()), e, source, Vec::new())
            }
            nu::ShellError::Generic(generic) => {
                let source = match &generic.site {
                    nu::ErrorSite::Location(location) => Some(location_source(location)),
                    nu::ErrorSite::Span(_) => None,
                };
                Self::of(NuValErrorKind::Generic, e, source, shells(&generic.inner))
            }
            nu::ShellError::LabeledError(labeled) => Self::from(labeled.as_ref()),
            nu::ShellError::Job(job) => Self::of(NuValErrorKind::Job(job.into()), e, None, Vec::new()),
            nu::ShellError::ChainedError(chain) => chained(chain),
            _ => Self::from_diagnostic(e),
        }
    }
}

#[cfg(feature = "nushell")]
impl From<&nu::LabeledError> for NuValError {
    fn from(e: &nu::LabeledError) -> Self {
        Self::of(NuValErrorKind::Labeled, e, None, shells(&e.inner))
    }
}

#[cfg(feature = "nushell")]
impl From<&nu::ParseError> for NuValError {
    fn from(e: &nu::ParseError) -> Self {
        Self::of(NuValErrorKind::Parse(e.into()), e, None, Vec::new())
    }
}

#[cfg(feature = "nushell")]
impl From<&nu::CompileError> for NuValError {
    fn from(e: &nu::CompileError) -> Self {
        Self::of(NuValErrorKind::Compile(e.into()), e, None, Vec::new())
    }
}

#[cfg(feature = "nushell")]
impl From<&nu::ConfigError> for NuValError {
    fn from(e: &nu::ConfigError) -> Self {
        match (ConfigErrorKind::of(e), e) {
            (Some(kind), _) => Self::of(NuValErrorKind::Config(kind), e, None, Vec::new()),
            (None, nu::ConfigError::ShellError(shell)) => Self::from(shell),
            (None, _) => Self::from_diagnostic(e),
        }
    }
}

#[cfg(feature = "nushell")]
impl From<&nu::ConfigWarning> for NuValError {
    fn from(e: &nu::ConfigWarning) -> Self {
        Self::of(NuValErrorKind::ConfigWarning(e.into()), e, None, Vec::new())
    }
}

#[cfg(feature = "nushell")]
impl From<&nu::ShellWarning> for NuValError {
    fn from(e: &nu::ShellWarning) -> Self {
        let inner = match e {
            nu::ShellWarning::InvalidConfig { warnings } => {
                warnings.iter().map(NuValError::from).collect()
            }
            _ => Vec::new(),
        };
        Self::of(NuValErrorKind::ShellWarning(e.into()), e, None, inner)
    }
}

#[cfg(feature = "nushell")]
impl From<&nu::ParseWarning> for NuValError {
    fn from(e: &nu::ParseWarning) -> Self {
        Self::of(NuValErrorKind::ParseWarning(e.into()), e, None, Vec::new())
    }
}

/// A span back in nushell, refused past this platform's usize or reversed.
#[cfg(feature = "nushell")]
fn nu_span(span: SpanData) -> ConvertResult<nu::Span> {
    let SpanData { start, end } = span;
    match (usize::try_from(start), usize::try_from(end)) {
        (Ok(s), Ok(e)) if s <= e => Ok(nu::Span::new(s, e)),
        _ => SpanSnafu { start, end }.fail(),
    }
}

#[cfg(feature = "nushell")]
impl TryFrom<NuValError> for nu::LabeledError {
    type Error = ConvertError;

    fn try_from(e: NuValError) -> ConvertResult<Self> {
        let labels = e
            .labels
            .into_iter()
            .map(|label| {
                Ok(nu::ErrorLabel {
                    text: label.text.unwrap_or_default(),
                    span: nu_span(label.span)?,
                })
            })
            .collect::<ConvertResult<Vec<_>>>()?;
        let inner = e
            .inner
            .into_iter()
            .map(nu::ShellError::try_from)
            .collect::<ConvertResult<Vec<_>>>()?;
        Ok(Self {
            msg: e.msg,
            labels: Box::new(labels),
            code: e.code,
            url: e.url,
            help: e.help,
            inner: Box::new(inner),
        })
    }
}

#[cfg(feature = "nushell")]
impl TryFrom<NuValError> for nu::ShellError {
    type Error = ConvertError;

    fn try_from(e: NuValError) -> ConvertResult<Self> {
        Ok(nu::ShellError::LabeledError(Box::new(e.try_into()?)))
    }
}

impl std::fmt::Display for NuValError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for NuValError {}
