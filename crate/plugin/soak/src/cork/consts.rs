use crate::*;

/// The span of a value that has no place in the caller's source.
pub(crate) const NOSPAN: nu::Span = nu::Span::unknown();
