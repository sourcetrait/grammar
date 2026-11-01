//! The nu type model: what a value nuval holds may be, as data.
use crate::*;

/// A nu type: the shape a value nushell holds may take.
#[cereal::derived(Data, Eq, Hash, Recursive, not(Serde))]
pub enum NuType {
    Any,
    Nothing,
    Bool,
    Int,
    Float,
    Number,
    String,
    /// A string naming a file or a directory.
    Path,
    /// A string naming a directory.
    Directory,
    Glob,
    Binary,
    Date,
    Duration,
    Filesize,
    Range,
    CellPath,
    List(Box<NuType>),
    /// A record's declared fields; none declared is an open record.
    Record(Vec<NuField>),
    /// A table's declared columns; none declared is an open table.
    Table(Vec<NuField>),
    OneOf(Vec<NuType>),
}

/// A record's field or a table's column: its name and its type.
#[cereal::derived(Data, Eq, Hash)]
pub struct NuField {
    pub name: String,
    pub ty: NuType,
}

impl NuField {
    /// A field named `name` of type `ty`.
    pub fn new(name: impl Into<String>, ty: NuType) -> Self {
        Self { name: name.into(), ty }
    }
}

impl NuType {
    /// The type a value of this one has: `path` and `directory` are strings.
    pub fn resolved(&self) -> NuType {
        match self {
            Self::Path | Self::Directory => Self::String,
            Self::List(inner) => Self::List(Box::new(inner.resolved())),
            Self::Record(fields) => Self::Record(resolved_fields(fields)),
            Self::Table(columns) => Self::Table(resolved_fields(columns)),
            Self::OneOf(members) => Self::OneOf(members.iter().map(Self::resolved).collect()),
            other => other.clone(),
        }
    }

    /// The type's name as nushell spells it.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Nothing => "nothing",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Float => "float",
            Self::Number => "number",
            Self::String => "string",
            Self::Path => "path",
            Self::Directory => "directory",
            Self::Glob => "glob",
            Self::Binary => "binary",
            Self::Date => "datetime",
            Self::Duration => "duration",
            Self::Filesize => "filesize",
            Self::Range => "range",
            Self::CellPath => "cell-path",
            Self::List(_) => "list",
            Self::Record(_) => "record",
            Self::Table(_) => "table",
            Self::OneOf(_) => "oneof",
        }
    }
}

/// Fields with each type resolved.
fn resolved_fields(fields: &[NuField]) -> Vec<NuField> {
    fields.iter().map(|f| NuField::new(f.name.clone(), f.ty.resolved())).collect()
}

/// A type's text on one line, as nushell's annotations spell it.
impl fmt::Display for NuType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_inline())
    }
}

impl FromStr for NuType {
    type Err = NuTypeError;

    fn from_str(text: &str) -> NuTypeResult<Self> {
        Self::from_text(text)
    }
}

/// A type serializes as its text, so NUON and every text format carry it as
/// nushell spells it; bitcode and rkyv carry its structure.
impl ser::Serialize for NuType {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_inline())
    }
}

impl<'de> de::Deserialize<'de> for NuType {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TypeText;

        impl de::Visitor<'_> for TypeText {
            type Value = NuType;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a nu type's text")
            }

            fn visit_str<E: de::Error>(self, text: &str) -> Result<NuType, E> {
                NuType::from_text(text).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(TypeText)
    }
}

/// A `.nutype` file: its documentation, the leading comment, and its type.
#[cereal::derived(Data, Eq, Hash)]
pub struct NuTypeFile {
    /// The leading comment's lines, `# ` dropped from each.
    pub doc: Vec<String>,
    pub ty: NuType,
}
