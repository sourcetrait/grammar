//! nutype's types to and from nushell's shapes and types.
use crate::*;

impl TryFrom<&nu::SyntaxShape> for NuType {
    type Error = NuTypeError;

    fn try_from(shape: &nu::SyntaxShape) -> NuTypeResult<Self> {
        use nu::SyntaxShape as S;
        Ok(match shape {
            S::Any => Self::Any,
            S::Nothing => Self::Nothing,
            S::Boolean => Self::Bool,
            S::Int => Self::Int,
            S::Float => Self::Float,
            S::Number => Self::Number,
            S::String => Self::String,
            S::Filepath => Self::Path,
            S::Directory => Self::Directory,
            S::GlobPattern => Self::Glob,
            S::Binary => Self::Binary,
            S::DateTime => Self::Date,
            S::Duration => Self::Duration,
            S::Filesize => Self::Filesize,
            S::Range => Self::Range,
            S::CellPath => Self::CellPath,
            S::List(element) => Self::List(Box::new(Self::try_from(element.as_ref())?)),
            S::Record(columns) => Self::Record(fields(columns.iter(), |s| Self::try_from(s))?),
            S::Table(columns) => Self::Table(fields(columns.iter(), |s| Self::try_from(s))?),
            S::OneOf(members) => Self::OneOf(members.iter().map(Self::try_from).collect::<NuTypeResult<_>>()?),
            other => return Err(NuTypeError::Unsupported { shape: other.to_string() }),
        })
    }
}

impl From<&NuType> for nu::SyntaxShape {
    fn from(ty: &NuType) -> Self {
        use nu::SyntaxShape as S;
        match ty {
            NuType::Any => S::Any,
            NuType::Nothing => S::Nothing,
            NuType::Bool => S::Boolean,
            NuType::Int => S::Int,
            NuType::Float => S::Float,
            NuType::Number => S::Number,
            NuType::String => S::String,
            NuType::Path => S::Filepath,
            NuType::Directory => S::Directory,
            NuType::Glob => S::GlobPattern,
            NuType::Binary => S::Binary,
            NuType::Date => S::DateTime,
            NuType::Duration => S::Duration,
            NuType::Filesize => S::Filesize,
            NuType::Range => S::Range,
            NuType::CellPath => S::CellPath,
            NuType::List(element) => S::List(Box::new(Self::from(element.as_ref()))),
            NuType::Record(fields) => S::Record(fields.iter().map(|f| (f.name.clone(), Self::from(&f.ty))).collect()),
            NuType::Table(columns) => S::Table(columns.iter().map(|f| (f.name.clone(), Self::from(&f.ty))).collect()),
            NuType::OneOf(members) => S::OneOf(members.iter().map(Self::from).collect()),
        }
    }
}

impl TryFrom<&nu::Type> for NuType {
    type Error = NuTypeError;

    fn try_from(ty: &nu::Type) -> NuTypeResult<Self> {
        use nu::Type as T;
        Ok(match ty {
            T::Any => Self::Any,
            T::Nothing => Self::Nothing,
            T::Bool => Self::Bool,
            T::Int => Self::Int,
            T::Float => Self::Float,
            T::Number => Self::Number,
            T::String => Self::String,
            T::Glob => Self::Glob,
            T::Binary => Self::Binary,
            T::Date => Self::Date,
            T::Duration => Self::Duration,
            T::Filesize => Self::Filesize,
            T::Range => Self::Range,
            T::CellPath => Self::CellPath,
            T::List(element) => Self::List(Box::new(Self::try_from(element.as_ref())?)),
            T::Record(columns) => Self::Record(fields(columns.iter(), |t| Self::try_from(t))?),
            T::Table(columns) => Self::Table(fields(columns.iter(), |t| Self::try_from(t))?),
            T::OneOf(members) => Self::OneOf(members.iter().map(Self::try_from).collect::<NuTypeResult<_>>()?),
            other => return Err(NuTypeError::Unsupported { shape: other.to_string() }),
        })
    }
}

/// nushell's type of a value of this one: `path` and `directory` are
/// strings, and a union's members merge as nushell merges them.
impl From<&NuType> for nu::Type {
    fn from(ty: &NuType) -> Self {
        use nu::Type as T;
        match ty {
            NuType::Any => T::Any,
            NuType::Nothing => T::Nothing,
            NuType::Bool => T::Bool,
            NuType::Int => T::Int,
            NuType::Float => T::Float,
            NuType::Number => T::Number,
            NuType::String | NuType::Path | NuType::Directory => T::String,
            NuType::Glob => T::Glob,
            NuType::Binary => T::Binary,
            NuType::Date => T::Date,
            NuType::Duration => T::Duration,
            NuType::Filesize => T::Filesize,
            NuType::Range => T::Range,
            NuType::CellPath => T::CellPath,
            NuType::List(element) => T::List(Box::new(Self::from(element.as_ref()))),
            NuType::Record(fields) => T::Record(fields.iter().map(|f| (f.name.clone(), Self::from(&f.ty))).collect()),
            NuType::Table(columns) => T::Table(columns.iter().map(|f| (f.name.clone(), Self::from(&f.ty))).collect()),
            NuType::OneOf(members) => T::OneOf(members.iter().map(Self::from).collect()),
        }
    }
}

/// nushell's columns as fields, each type converted.
fn fields<'a, T: 'a>(
    columns: impl Iterator<Item = &'a (String, T)>,
    convert: impl Fn(&T) -> NuTypeResult<NuType>,
) -> NuTypeResult<Vec<NuField>> {
    columns
        .map(|(name, inner)| convert(inner).map(|ty| NuField::new(name.clone(), ty)).map_err(|e| e.within(name)))
        .collect()
}
