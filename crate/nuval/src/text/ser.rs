//! Any serde type as a NuVal, and through it as NUON text.
use crate::*;

/// The value serde writes `value` as: a struct or a map a record, a sequence
/// a list, a unit variant its name, any other variant a one-key record.
pub fn to_nuval<T: ser::Serialize + ?Sized>(value: &T) -> NuonResult<NuVal> {
    value.serialize(Serializer)
}

/// `value` as NUON text in `style`.
pub fn to_nuon<T: ser::Serialize + ?Sized>(value: &T, style: NuonStyle) -> NuonResult<String> {
    Ok(to_nuval(value)?.to_nuon(style))
}

/// serde's writer into a NuVal.
struct Serializer;

/// An int past i64, which nushell's ints don't reach.
fn past_int(value: impl fmt::Display) -> NuonError {
    NuonError::convert(format!("{value} is past a 64-bit int"))
}

impl ser::Serializer for Serializer {
    type Ok = NuVal;
    type Error = NuonError;
    type SerializeSeq = Items;
    type SerializeTuple = Items;
    type SerializeTupleStruct = Items;
    type SerializeTupleVariant = Variant<Items>;
    type SerializeMap = Entries;
    type SerializeStruct = Entries;
    type SerializeStructVariant = Variant<Entries>;

    fn serialize_bool(self, v: bool) -> NuonResult<NuVal> {
        Ok(NuVal::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> NuonResult<NuVal> {
        Ok(NuVal::Int(i64::from(v)))
    }

    fn serialize_i16(self, v: i16) -> NuonResult<NuVal> {
        Ok(NuVal::Int(i64::from(v)))
    }

    fn serialize_i32(self, v: i32) -> NuonResult<NuVal> {
        Ok(NuVal::Int(i64::from(v)))
    }

    fn serialize_i64(self, v: i64) -> NuonResult<NuVal> {
        Ok(NuVal::Int(v))
    }

    fn serialize_i128(self, v: i128) -> NuonResult<NuVal> {
        i64::try_from(v).map(NuVal::Int).map_err(|_| past_int(v))
    }

    fn serialize_u8(self, v: u8) -> NuonResult<NuVal> {
        Ok(NuVal::Int(i64::from(v)))
    }

    fn serialize_u16(self, v: u16) -> NuonResult<NuVal> {
        Ok(NuVal::Int(i64::from(v)))
    }

    fn serialize_u32(self, v: u32) -> NuonResult<NuVal> {
        Ok(NuVal::Int(i64::from(v)))
    }

    fn serialize_u64(self, v: u64) -> NuonResult<NuVal> {
        i64::try_from(v).map(NuVal::Int).map_err(|_| past_int(v))
    }

    fn serialize_u128(self, v: u128) -> NuonResult<NuVal> {
        i64::try_from(v).map(NuVal::Int).map_err(|_| past_int(v))
    }

    fn serialize_f32(self, v: f32) -> NuonResult<NuVal> {
        Ok(NuVal::Float(f64::from(v)))
    }

    fn serialize_f64(self, v: f64) -> NuonResult<NuVal> {
        Ok(NuVal::Float(v))
    }

    fn serialize_char(self, v: char) -> NuonResult<NuVal> {
        Ok(NuVal::String(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> NuonResult<NuVal> {
        Ok(NuVal::String(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> NuonResult<NuVal> {
        Ok(NuVal::Binary(v.to_vec()))
    }

    fn serialize_none(self) -> NuonResult<NuVal> {
        Ok(NuVal::Nothing)
    }

    fn serialize_some<T: ser::Serialize + ?Sized>(self, value: &T) -> NuonResult<NuVal> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> NuonResult<NuVal> {
        Ok(NuVal::Nothing)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> NuonResult<NuVal> {
        Ok(NuVal::Nothing)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> NuonResult<NuVal> {
        Ok(NuVal::String(variant.to_string()))
    }

    fn serialize_newtype_struct<T: ser::Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> NuonResult<NuVal> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ser::Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> NuonResult<NuVal> {
        let value = value.serialize(Serializer).map_err(|e| e.within(variant))?;
        Ok(NuVal::Record(vec![(variant.to_string(), value)]))
    }

    fn serialize_seq(self, len: Option<usize>) -> NuonResult<Items> {
        Ok(Items(Vec::with_capacity(len.unwrap_or(0))))
    }

    fn serialize_tuple(self, len: usize) -> NuonResult<Items> {
        Ok(Items(Vec::with_capacity(len)))
    }

    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> NuonResult<Items> {
        Ok(Items(Vec::with_capacity(len)))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> NuonResult<Variant<Items>> {
        Ok(Variant { name: variant, inner: Items(Vec::with_capacity(len)) })
    }

    fn serialize_map(self, len: Option<usize>) -> NuonResult<Entries> {
        Ok(Entries { entries: Vec::with_capacity(len.unwrap_or(0)), key: None })
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> NuonResult<Entries> {
        Ok(Entries { entries: Vec::with_capacity(len), key: None })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> NuonResult<Variant<Entries>> {
        Ok(Variant { name: variant, inner: Entries { entries: Vec::with_capacity(len), key: None } })
    }
}

/// A list's items as they are written.
pub(crate) struct Items(Vec<NuVal>);

impl Items {
    fn push<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> NuonResult<()> {
        let index = self.0.len();
        self.0.push(value.serialize(Serializer).map_err(|e| e.within(index))?);
        Ok(())
    }
}

impl ser::SerializeSeq for Items {
    type Ok = NuVal;
    type Error = NuonError;

    fn serialize_element<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> NuonResult<()> {
        self.push(value)
    }

    fn end(self) -> NuonResult<NuVal> {
        Ok(NuVal::List(self.0))
    }
}

impl ser::SerializeTuple for Items {
    type Ok = NuVal;
    type Error = NuonError;

    fn serialize_element<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> NuonResult<()> {
        self.push(value)
    }

    fn end(self) -> NuonResult<NuVal> {
        Ok(NuVal::List(self.0))
    }
}

impl ser::SerializeTupleStruct for Items {
    type Ok = NuVal;
    type Error = NuonError;

    fn serialize_field<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> NuonResult<()> {
        self.push(value)
    }

    fn end(self) -> NuonResult<NuVal> {
        Ok(NuVal::List(self.0))
    }
}

/// A record's entries as they are written, a map's key waiting for its value.
pub(crate) struct Entries {
    entries: Vec<(String, NuVal)>,
    key: Option<String>,
}

impl Entries {
    fn push<T: ser::Serialize + ?Sized>(&mut self, key: String, value: &T) -> NuonResult<()> {
        if self.entries.iter().any(|(k, _)| *k == key) {
            return Err(NuonError::convert(format!("the key `{key}` appears twice")));
        }
        let value = value.serialize(Serializer).map_err(|e| e.within(&key))?;
        self.entries.push((key, value));
        Ok(())
    }
}

impl ser::SerializeMap for Entries {
    type Ok = NuVal;
    type Error = NuonError;

    fn serialize_key<T: ser::Serialize + ?Sized>(&mut self, key: &T) -> NuonResult<()> {
        self.key = Some(key.serialize(KeySerializer)?);
        Ok(())
    }

    fn serialize_value<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> NuonResult<()> {
        let key = self.key.take().ok_or_else(|| NuonError::convert("a map's value came before its key"))?;
        self.push(key, value)
    }

    fn end(self) -> NuonResult<NuVal> {
        Ok(NuVal::Record(self.entries))
    }
}

impl ser::SerializeStruct for Entries {
    type Ok = NuVal;
    type Error = NuonError;

    fn serialize_field<T: ser::Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> NuonResult<()> {
        self.push(key.to_string(), value)
    }

    fn end(self) -> NuonResult<NuVal> {
        Ok(NuVal::Record(self.entries))
    }
}

/// A tuple or struct variant: its fields, then wrapped in a record under its
/// name.
pub(crate) struct Variant<T> {
    name: &'static str,
    inner: T,
}

impl ser::SerializeTupleVariant for Variant<Items> {
    type Ok = NuVal;
    type Error = NuonError;

    fn serialize_field<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> NuonResult<()> {
        self.inner.push(value).map_err(|e| e.within(self.name))
    }

    fn end(self) -> NuonResult<NuVal> {
        Ok(NuVal::Record(vec![(self.name.to_string(), NuVal::List(self.inner.0))]))
    }
}

impl ser::SerializeStructVariant for Variant<Entries> {
    type Ok = NuVal;
    type Error = NuonError;

    fn serialize_field<T: ser::Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> NuonResult<()> {
        self.inner.push(key.to_string(), value).map_err(|e| e.within(self.name))
    }

    fn end(self) -> NuonResult<NuVal> {
        Ok(NuVal::Record(vec![(self.name.to_string(), NuVal::Record(self.inner.entries))]))
    }
}

/// A map's key as a record's string key: strings, chars, numbers, bools, and
/// unit variants, as serde_json takes them.
struct KeySerializer;

/// A key nushell's records can't hold.
fn not_a_key() -> NuonError {
    NuonError::convert("a record's key is a string, a number, a bool, or a unit variant")
}

impl ser::Serializer for KeySerializer {
    type Ok = String;
    type Error = NuonError;
    type SerializeSeq = ser::Impossible<String, NuonError>;
    type SerializeTuple = ser::Impossible<String, NuonError>;
    type SerializeTupleStruct = ser::Impossible<String, NuonError>;
    type SerializeTupleVariant = ser::Impossible<String, NuonError>;
    type SerializeMap = ser::Impossible<String, NuonError>;
    type SerializeStruct = ser::Impossible<String, NuonError>;
    type SerializeStructVariant = ser::Impossible<String, NuonError>;

    fn serialize_bool(self, v: bool) -> NuonResult<String> {
        Ok(v.to_string())
    }

    fn serialize_i8(self, v: i8) -> NuonResult<String> {
        Ok(v.to_string())
    }

    fn serialize_i16(self, v: i16) -> NuonResult<String> {
        Ok(v.to_string())
    }

    fn serialize_i32(self, v: i32) -> NuonResult<String> {
        Ok(v.to_string())
    }

    fn serialize_i64(self, v: i64) -> NuonResult<String> {
        Ok(v.to_string())
    }

    fn serialize_i128(self, v: i128) -> NuonResult<String> {
        Ok(v.to_string())
    }

    fn serialize_u8(self, v: u8) -> NuonResult<String> {
        Ok(v.to_string())
    }

    fn serialize_u16(self, v: u16) -> NuonResult<String> {
        Ok(v.to_string())
    }

    fn serialize_u32(self, v: u32) -> NuonResult<String> {
        Ok(v.to_string())
    }

    fn serialize_u64(self, v: u64) -> NuonResult<String> {
        Ok(v.to_string())
    }

    fn serialize_u128(self, v: u128) -> NuonResult<String> {
        Ok(v.to_string())
    }

    fn serialize_f32(self, _v: f32) -> NuonResult<String> {
        Err(not_a_key())
    }

    fn serialize_f64(self, _v: f64) -> NuonResult<String> {
        Err(not_a_key())
    }

    fn serialize_char(self, v: char) -> NuonResult<String> {
        Ok(v.to_string())
    }

    fn serialize_str(self, v: &str) -> NuonResult<String> {
        Ok(v.to_string())
    }

    fn serialize_bytes(self, _v: &[u8]) -> NuonResult<String> {
        Err(not_a_key())
    }

    fn serialize_none(self) -> NuonResult<String> {
        Err(not_a_key())
    }

    fn serialize_some<T: ser::Serialize + ?Sized>(self, _value: &T) -> NuonResult<String> {
        Err(not_a_key())
    }

    fn serialize_unit(self) -> NuonResult<String> {
        Err(not_a_key())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> NuonResult<String> {
        Err(not_a_key())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> NuonResult<String> {
        Ok(variant.to_string())
    }

    fn serialize_newtype_struct<T: ser::Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> NuonResult<String> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ser::Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> NuonResult<String> {
        Err(not_a_key())
    }

    fn serialize_seq(self, _len: Option<usize>) -> NuonResult<Self::SerializeSeq> {
        Err(not_a_key())
    }

    fn serialize_tuple(self, _len: usize) -> NuonResult<Self::SerializeTuple> {
        Err(not_a_key())
    }

    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> NuonResult<Self::SerializeTupleStruct> {
        Err(not_a_key())
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> NuonResult<Self::SerializeTupleVariant> {
        Err(not_a_key())
    }

    fn serialize_map(self, _len: Option<usize>) -> NuonResult<Self::SerializeMap> {
        Err(not_a_key())
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> NuonResult<Self::SerializeStruct> {
        Err(not_a_key())
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> NuonResult<Self::SerializeStructVariant> {
        Err(not_a_key())
    }
}
