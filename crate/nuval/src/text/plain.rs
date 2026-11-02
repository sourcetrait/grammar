//! An `Option<NuVal>` field through serde as the value it holds rather than as
//! its variant: `#[serde(default, with = "nuval::plain")]`.
use crate::*;

/// The field's value written as the value it holds, or as nothing.
pub fn serialize<S: ser::Serializer>(value: &Option<NuVal>, serializer: S) -> Result<S::Ok, S::Error> {
    match value {
        Some(value) => serializer.serialize_some(&Plain(value)),
        None => serializer.serialize_none(),
    }
}

/// The field's value read back as the NuVal it holds, nothing as none.
pub fn deserialize<'de, D: de::Deserializer<'de>>(deserializer: D) -> Result<Option<NuVal>, D::Error> {
    deserializer.deserialize_option(OptionVisitor)
}

/// A NuVal written as its value, each kind as `from_nuval` hands it to serde.
struct Plain<'a>(&'a NuVal);

impl ser::Serialize for Plain<'_> {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use ser::{SerializeMap, SerializeSeq};
        match self.0 {
            NuVal::Bool(b) => serializer.serialize_bool(*b),
            NuVal::Int(i) | NuVal::Filesize(i) | NuVal::Duration(i) => serializer.serialize_i64(*i),
            NuVal::Float(f) => serializer.serialize_f64(*f),
            NuVal::String(s) => serializer.serialize_str(s),
            NuVal::Glob(glob) => serializer.serialize_str(&glob.glob),
            value @ (NuVal::Date(_) | NuVal::Range(_) | NuVal::CellPath(_)) => {
                serializer.serialize_str(&value.to_nuon(NuonStyle::Raw))
            }
            NuVal::Record(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, &Plain(value))?;
                }
                map.end()
            }
            NuVal::List(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(&Plain(item))?;
                }
                seq.end()
            }
            NuVal::Binary(bytes) => serializer.serialize_bytes(bytes),
            NuVal::Nothing => serializer.serialize_unit(),
        }
    }
}

/// An optional value: nothing as none, any other value as its NuVal.
struct OptionVisitor;

impl<'de> de::Visitor<'de> for OptionVisitor {
    type Value = Option<NuVal>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a nushell value or nothing")
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_some<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(ValueVisitor).map(Some)
    }
}

/// A value, each serde kind read as its NuVal.
struct ValueVisitor;

impl<'de> de::Visitor<'de> for ValueVisitor {
    type Value = NuVal;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a nushell value")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<NuVal, E> {
        Ok(NuVal::Bool(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<NuVal, E> {
        Ok(NuVal::Int(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<NuVal, E> {
        i64::try_from(v).map(NuVal::Int).map_err(|_| E::custom(format!("{v} is past an int's range")))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<NuVal, E> {
        Ok(NuVal::Float(v))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<NuVal, E> {
        Ok(NuVal::String(v.to_string()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<NuVal, E> {
        Ok(NuVal::String(v))
    }

    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<NuVal, E> {
        Ok(NuVal::Binary(v.to_vec()))
    }

    fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<NuVal, E> {
        Ok(NuVal::Binary(v))
    }

    fn visit_none<E: de::Error>(self) -> Result<NuVal, E> {
        Ok(NuVal::Nothing)
    }

    fn visit_unit<E: de::Error>(self) -> Result<NuVal, E> {
        Ok(NuVal::Nothing)
    }

    fn visit_some<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<NuVal, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<NuVal, A::Error> {
        let mut items = Vec::new();
        while let Some(Read(item)) = seq.next_element()? {
            items.push(item);
        }
        Ok(NuVal::List(items))
    }

    fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<NuVal, A::Error> {
        let mut entries = Vec::new();
        while let Some((key, Read(value))) = map.next_entry::<String, Read>()? {
            entries.push((key, value));
        }
        Ok(NuVal::Record(entries))
    }
}

/// A value read through serde as its NuVal.
struct Read(NuVal);

impl<'de> de::Deserialize<'de> for Read {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ValueVisitor).map(Read)
    }
}
