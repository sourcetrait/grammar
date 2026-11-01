//! A NuVal read by its parts: a record's fields, a list's items, and a
//! scalar's value.
use crate::*;

/// What an index finds where there is nothing.
static NOTHING: NuVal = NuVal::Nothing;

impl NuVal {
    /// A record's field, none for a missing field or another kind.
    pub fn get(&self, key: &str) -> Option<&NuVal> {
        self.as_record()?.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// A record's fields in order.
    pub fn as_record(&self) -> Option<&[(String, NuVal)]> {
        match self {
            Self::Record(fields) => Some(fields),
            _ => None,
        }
    }

    /// A list's items.
    pub fn as_list(&self) -> Option<&[NuVal]> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }

    /// A string's text.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    /// An int.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(int) => Some(*int),
            _ => None,
        }
    }

    /// An int at zero or above.
    pub fn as_u64(&self) -> Option<u64> {
        self.as_i64().and_then(|int| u64::try_from(int).ok())
    }

    /// A float, or an int as one.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(float) => Some(*float),
            Self::Int(int) => Some(*int as f64),
            _ => None,
        }
    }

    /// A bool.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Whether the value is nothing.
    pub fn is_nothing(&self) -> bool {
        matches!(self, Self::Nothing)
    }
}

impl std::ops::Index<&str> for NuVal {
    type Output = NuVal;

    /// A record's field, nothing where there is none.
    fn index(&self, key: &str) -> &NuVal {
        self.get(key).unwrap_or(&NOTHING)
    }
}

impl std::ops::Index<usize> for NuVal {
    type Output = NuVal;

    /// A list's item, nothing where there is none.
    fn index(&self, index: usize) -> &NuVal {
        self.as_list().and_then(|items| items.get(index)).unwrap_or(&NOTHING)
    }
}

impl PartialEq<str> for NuVal {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == Some(other)
    }
}

impl PartialEq<&str> for NuVal {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == Some(*other)
    }
}

impl PartialEq<String> for NuVal {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == Some(other.as_str())
    }
}

impl PartialEq<i64> for NuVal {
    fn eq(&self, other: &i64) -> bool {
        self.as_i64() == Some(*other)
    }
}

impl PartialEq<bool> for NuVal {
    fn eq(&self, other: &bool) -> bool {
        self.as_bool() == Some(*other)
    }
}

impl From<&str> for NuVal {
    fn from(text: &str) -> Self {
        Self::String(text.to_string())
    }
}

impl From<String> for NuVal {
    fn from(text: String) -> Self {
        Self::String(text)
    }
}

impl From<i64> for NuVal {
    fn from(int: i64) -> Self {
        Self::Int(int)
    }
}

impl From<bool> for NuVal {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl fmt::Display for NuVal {
    /// The value as raw NUON, on one line.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_nuon(NuonStyle::Raw))
    }
}
