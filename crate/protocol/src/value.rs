//! Values and their errors on the wire, held to the wire's bounds.
use crate::*;

/// The bounds a value or an error is held to on the wire.
#[cereal::derived(Data, Copy, Eq)]
pub struct ValueLimits {
    /// The deepest nesting of lists, records, and inner errors.
    pub depth: u32,
    /// The most elements: list items, record fields, members, labels, inners.
    pub elements: u64,
    /// The most bytes of text and binary together.
    pub bytes: u64,
}

impl ValueLimits {
    /// Version 1's bounds: depth 64, 100,000 elements, 32 MiB of bytes.
    pub const V1: Self = Self {
        depth: 64,
        elements: 100_000,
        bytes: 32 * 1024 * 1024,
    };
}

/// A value or an error past one of its bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueLimitError {
    /// Nested deeper than the bound.
    Depth { limit: u32 },
    /// More elements than the bound.
    Elements { limit: u64 },
    /// More bytes of text and binary than the bound.
    Bytes { limit: u64 },
}

/// Payload-less mirror of `ValueLimitError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueLimitErrorKind {
    Depth,
    Elements,
    Bytes,
}

impl ValueLimitError {
    /// The bound's kind.
    pub fn kind(&self) -> ValueLimitErrorKind {
        match self {
            Self::Depth { .. } => ValueLimitErrorKind::Depth,
            Self::Elements { .. } => ValueLimitErrorKind::Elements,
            Self::Bytes { .. } => ValueLimitErrorKind::Bytes,
        }
    }
}

impl fmt::Display for ValueLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Depth { limit } => write!(f, "nested deeper than {limit}"),
            Self::Elements { limit } => write!(f, "more than {limit} elements"),
            Self::Bytes { limit } => write!(f, "more than {limit} bytes of text and binary"),
        }
    }
}

impl std::error::Error for ValueLimitError {}

/// The running totals of one check against its bounds.
struct Tally {
    limits: ValueLimits,
    elements: u64,
    bytes: u64,
}

impl Tally {
    /// A check's totals, starting at nothing.
    fn new(limits: ValueLimits) -> Self {
        Self { limits, elements: 0, bytes: 0 }
    }

    /// A container at a nesting depth, refused past the bound.
    fn depth(&self, depth: u32) -> Result<(), ValueLimitError> {
        if depth > self.limits.depth {
            return Err(ValueLimitError::Depth { limit: self.limits.depth });
        }
        Ok(())
    }

    /// Elements counted, refused once the total passes the bound.
    fn elements(&mut self, n: usize) -> Result<(), ValueLimitError> {
        self.elements = self.elements.saturating_add(wide(n));
        if self.elements > self.limits.elements {
            return Err(ValueLimitError::Elements { limit: self.limits.elements });
        }
        Ok(())
    }

    /// Bytes counted, refused once the total passes the bound.
    fn bytes(&mut self, n: usize) -> Result<(), ValueLimitError> {
        self.bytes = self.bytes.saturating_add(wide(n));
        if self.bytes > self.limits.bytes {
            return Err(ValueLimitError::Bytes { limit: self.limits.bytes });
        }
        Ok(())
    }

    /// Optional text's bytes counted.
    fn text(&mut self, text: Option<&String>) -> Result<(), ValueLimitError> {
        self.bytes(text.map_or(0, String::len))
    }
}

/// A value held to its bounds, walked without recursion.
pub fn check_value(value: &NuVal, limits: ValueLimits) -> Result<(), ValueLimitError> {
    let mut tally = Tally::new(limits);
    let mut stack = vec![(value, 0u32)];
    while let Some((value, outer)) = stack.pop() {
        match value {
            NuVal::String(text) => tally.bytes(text.len())?,
            NuVal::Glob(glob) => tally.bytes(glob.glob.len())?,
            NuVal::Binary(bytes) => tally.bytes(bytes.len())?,
            NuVal::CellPath(members) => {
                tally.elements(members.len())?;
                for member in members {
                    if let CellPathMemberData::String { value, .. } = member {
                        tally.bytes(value.len())?;
                    }
                }
            }
            NuVal::List(items) => {
                tally.depth(outer + 1)?;
                tally.elements(items.len())?;
                stack.extend(items.iter().map(|item| (item, outer + 1)));
            }
            NuVal::Record(fields) => {
                tally.depth(outer + 1)?;
                tally.elements(fields.len())?;
                for (column, field) in fields {
                    tally.bytes(column.len())?;
                    stack.push((field, outer + 1));
                }
            }
            NuVal::Bool(_)
            | NuVal::Int(_)
            | NuVal::Float(_)
            | NuVal::Filesize(_)
            | NuVal::Duration(_)
            | NuVal::Date(_)
            | NuVal::Range(_)
            | NuVal::Nothing => {}
        }
    }
    Ok(())
}

/// An error held to its bounds, its inner errors walked without recursion.
pub fn check_error(error: &NuValError, limits: ValueLimits) -> Result<(), ValueLimitError> {
    let mut tally = Tally::new(limits);
    let mut stack = vec![(error, 1u32)];
    while let Some((error, depth)) = stack.pop() {
        tally.depth(depth)?;
        tally.bytes(error.msg.len())?;
        tally.text(error.code.as_ref())?;
        tally.text(error.help.as_ref())?;
        tally.text(error.url.as_ref())?;
        tally.text(error.cause.as_ref())?;
        if let Some(source) = &error.source {
            tally.text(source.name.as_ref())?;
            tally.bytes(source.text.len())?;
        }
        tally.elements(error.labels.len())?;
        for label in &error.labels {
            tally.text(label.text.as_ref())?;
        }
        tally.elements(error.inner.len())?;
        stack.extend(error.inner.iter().map(|inner| (inner, depth + 1)));
    }
    Ok(())
}
