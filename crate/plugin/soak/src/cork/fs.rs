use crate::*;

/// A path and the span of the argument it came from.
#[derive(Debug)]
pub(crate) struct PathSpan<'a> {
    pub(crate) path: &'a Path,
    pub(crate) span: nu::Span,
}
