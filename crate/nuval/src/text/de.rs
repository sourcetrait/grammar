//! Any serde type from the NuVal it was written as, and through it from NUON
//! text.
use crate::*;

/// A serde type from its NuVal.
pub fn from_nuval<T: de::DeserializeOwned>(value: NuVal) -> NuonResult<T> {
    T::deserialize(value)
}

/// A serde type from NUON text.
pub fn from_nuon<T: de::DeserializeOwned>(text: &str) -> NuonResult<T> {
    from_nuval(NuVal::from_nuon(text)?)
}

impl<'de> de::Deserializer<'de> for NuVal {
    type Error = NuonError;

    fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> NuonResult<V::Value> {
        match self {
            NuVal::Bool(b) => visitor.visit_bool(b),
            NuVal::Int(i) | NuVal::Filesize(i) | NuVal::Duration(i) => visitor.visit_i64(i),
            NuVal::Float(f) => visitor.visit_f64(f),
            NuVal::String(s) => visitor.visit_string(s),
            NuVal::Glob(glob) => visitor.visit_string(glob.glob),
            value @ (NuVal::Date(_) | NuVal::Range(_) | NuVal::CellPath(_)) => {
                visitor.visit_string(value.to_nuon(NuonStyle::Raw))
            }
            NuVal::Record(entries) => visitor.visit_map(Entries::new(entries)),
            NuVal::List(items) => visitor.visit_seq(Items::new(items)),
            NuVal::Binary(bytes) => visitor.visit_byte_buf(bytes),
            NuVal::Nothing => visitor.visit_unit(),
        }
    }

    fn deserialize_option<V: de::Visitor<'de>>(self, visitor: V) -> NuonResult<V::Value> {
        match self {
            NuVal::Nothing => visitor.visit_none(),
            value => visitor.visit_some(value),
        }
    }

    fn deserialize_newtype_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> NuonResult<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> NuonResult<V::Value> {
        match self {
            NuVal::String(name) => visitor.visit_enum(Variant { name, value: None }),
            NuVal::Record(mut entries) if entries.len() == 1 => {
                let (name, value) = entries.remove(0);
                visitor.visit_enum(Variant { name, value: Some(value) })
            }
            _ => Err(NuonError::convert("a variant is its name, or a record of one key holding its value")),
        }
    }

    fn deserialize_ignored_any<V: de::Visitor<'de>>(self, visitor: V) -> NuonResult<V::Value> {
        visitor.visit_unit()
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        unit unit_struct seq tuple tuple_struct map struct identifier
    }
}

/// A list's items as they are read.
struct Items {
    items: std::vec::IntoIter<NuVal>,
    index: usize,
}

impl Items {
    fn new(items: Vec<NuVal>) -> Self {
        Self { items: items.into_iter(), index: 0 }
    }
}

impl<'de> de::SeqAccess<'de> for Items {
    type Error = NuonError;

    fn next_element_seed<T: de::DeserializeSeed<'de>>(&mut self, seed: T) -> NuonResult<Option<T::Value>> {
        let Some(item) = self.items.next() else {
            return Ok(None);
        };
        let index = self.index;
        self.index += 1;
        seed.deserialize(item).map(Some).map_err(|e| e.within(index))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.items.len())
    }
}

/// A record's entries as they are read, the last key's value held for its
/// turn.
struct Entries {
    entries: std::vec::IntoIter<(String, NuVal)>,
    value: Option<(String, NuVal)>,
}

impl Entries {
    fn new(entries: Vec<(String, NuVal)>) -> Self {
        Self { entries: entries.into_iter(), value: None }
    }
}

impl<'de> de::MapAccess<'de> for Entries {
    type Error = NuonError;

    fn next_key_seed<K: de::DeserializeSeed<'de>>(&mut self, seed: K) -> NuonResult<Option<K::Value>> {
        let Some((key, value)) = self.entries.next() else {
            return Ok(None);
        };
        let read = seed.deserialize(Key(key.clone())).map_err(|e| e.within(&key))?;
        self.value = Some((key, value));
        Ok(Some(read))
    }

    fn next_value_seed<V: de::DeserializeSeed<'de>>(&mut self, seed: V) -> NuonResult<V::Value> {
        let (key, value) = self.value.take().ok_or_else(|| NuonError::convert("a value read before its key"))?;
        seed.deserialize(value).map_err(|e| e.within(key))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.entries.len())
    }
}

/// A variant by its name, with the value a record of one key holds.
struct Variant {
    name: String,
    value: Option<NuVal>,
}

impl<'de> de::EnumAccess<'de> for Variant {
    type Error = NuonError;
    type Variant = Self;

    fn variant_seed<V: de::DeserializeSeed<'de>>(self, seed: V) -> NuonResult<(V::Value, Self)> {
        let tag = seed.deserialize(NuVal::String(self.name.clone()))?;
        Ok((tag, self))
    }
}

impl<'de> de::VariantAccess<'de> for Variant {
    type Error = NuonError;

    fn unit_variant(self) -> NuonResult<()> {
        match self.value {
            None | Some(NuVal::Nothing) => Ok(()),
            Some(_) => Err(NuonError::convert(format!("the variant `{}` holds no value", self.name))),
        }
    }

    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(self, seed: T) -> NuonResult<T::Value> {
        let name = self.name;
        match self.value {
            Some(value) => seed.deserialize(value).map_err(|e| e.within(name)),
            None => Err(NuonError::convert(format!("the variant `{name}` holds a value"))),
        }
    }

    fn tuple_variant<V: de::Visitor<'de>>(self, _len: usize, visitor: V) -> NuonResult<V::Value> {
        let name = self.name;
        match self.value {
            Some(NuVal::List(items)) => visitor.visit_seq(Items::new(items)).map_err(|e| e.within(name)),
            _ => Err(NuonError::convert(format!("the variant `{name}` holds a list"))),
        }
    }

    fn struct_variant<V: de::Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> NuonResult<V::Value> {
        let name = self.name;
        match self.value {
            Some(NuVal::Record(entries)) => visitor.visit_map(Entries::new(entries)).map_err(|e| e.within(name)),
            _ => Err(NuonError::convert(format!("the variant `{name}` holds a record"))),
        }
    }
}

/// A record's key, read as a string, or as a number or a bool where the
/// type asks for one.
struct Key(String);

impl Key {
    fn parsed<T: std::str::FromStr>(&self, kind: &str) -> NuonResult<T> {
        self.0.parse().map_err(|_| NuonError::convert(format!("the key `{}` isn't a {kind}", self.0)))
    }
}

impl<'de> de::Deserializer<'de> for Key {
    type Error = NuonError;

    fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> NuonResult<V::Value> {
        visitor.visit_string(self.0)
    }

    fn deserialize_bool<V: de::Visitor<'de>>(self, visitor: V) -> NuonResult<V::Value> {
        visitor.visit_bool(self.parsed("bool")?)
    }

    fn deserialize_i8<V: de::Visitor<'de>>(self, visitor: V) -> NuonResult<V::Value> {
        visitor.visit_i8(self.parsed("number")?)
    }

    fn deserialize_i16<V: de::Visitor<'de>>(self, visitor: V) -> NuonResult<V::Value> {
        visitor.visit_i16(self.parsed("number")?)
    }

    fn deserialize_i32<V: de::Visitor<'de>>(self, visitor: V) -> NuonResult<V::Value> {
        visitor.visit_i32(self.parsed("number")?)
    }

    fn deserialize_i64<V: de::Visitor<'de>>(self, visitor: V) -> NuonResult<V::Value> {
        visitor.visit_i64(self.parsed("number")?)
    }

    fn deserialize_u8<V: de::Visitor<'de>>(self, visitor: V) -> NuonResult<V::Value> {
        visitor.visit_u8(self.parsed("number")?)
    }

    fn deserialize_u16<V: de::Visitor<'de>>(self, visitor: V) -> NuonResult<V::Value> {
        visitor.visit_u16(self.parsed("number")?)
    }

    fn deserialize_u32<V: de::Visitor<'de>>(self, visitor: V) -> NuonResult<V::Value> {
        visitor.visit_u32(self.parsed("number")?)
    }

    fn deserialize_u64<V: de::Visitor<'de>>(self, visitor: V) -> NuonResult<V::Value> {
        visitor.visit_u64(self.parsed("number")?)
    }

    fn deserialize_newtype_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> NuonResult<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> NuonResult<V::Value> {
        visitor.visit_enum(Variant { name: self.0, value: None })
    }

    serde::forward_to_deserialize_any! {
        i128 u128 f32 f64 char str string bytes byte_buf option unit unit_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}
