//! A golden file: NUON under the crate, written from the actual value first
//! under `BLESS=1`, read back through nuval, and held to its `.nutype`.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;

use nuval::{NuTypeFile, NuVal, NuonStyle};

/// The golden at `file`, its model at `model`, both under the crate; the
/// file written from `actual` first under `BLESS=1`.
pub fn read(file: &str, model: &str, actual: &NuVal) -> NuVal {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join(file);
    if std::env::var("BLESS").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().expect("the golden's directory")).expect("created");
        std::fs::write(&path, format!("{}\n", actual.to_nuon(NuonStyle::Spaces(2)))).expect("the golden written");
    }
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {file}: {e}; BLESS=1 writes it"));
    let golden = NuVal::from_nuon(&text).unwrap_or_else(|e| panic!("{file}: {e}"));
    let model_text = std::fs::read_to_string(root.join(model)).unwrap_or_else(|e| panic!("read {model}: {e}"));
    let model = NuTypeFile::from_text(&model_text).unwrap_or_else(|e| panic!("{model}: {e}"));
    model.ty.check(&golden).unwrap_or_else(|e| panic!("{file} against its model: {e}"));
    golden
}

/// A table's row: its columns in order.
pub fn row(columns: impl IntoIterator<Item = (&'static str, NuVal)>) -> NuVal {
    NuVal::Record(columns.into_iter().map(|(name, value)| (name.to_string(), value)).collect())
}

/// One golden vector: its value's type, as the fixture driver names it, and
/// its pinned bitcode.
pub struct Vector {
    pub ty: String,
    pub bytes: Vec<u8>,
}

/// A value's golden vector.
pub fn vector<T: bitcode::Encode + ?Sized>(value: &T) -> Vector {
    Vector { ty: type_name::<T>(), bytes: bitcode::encode(value) }
}

/// `T`'s name with every module path dropped, `Result<NuVal, EngineError>`.
pub fn type_name<T: ?Sized>() -> String {
    let mut out = String::new();
    let mut word = String::new();
    let mut chars = std::any::type_name::<T>().chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
        } else if c == ':' && chars.peek() == Some(&':') {
            chars.next();
            word.clear();
        } else {
            out.push_str(&word);
            word.clear();
            out.push(c);
        }
    }
    out + &word
}

/// Golden bitcode vectors as `Vectors.nutype`'s table: each name, its type,
/// and its bytes in lowercase hex.
pub fn hex_table(vectors: BTreeMap<String, Vector>) -> NuVal {
    NuVal::List(
        vectors
            .into_iter()
            .map(|(name, vector)| {
                let hex: String = vector.bytes.iter().map(|b| format!("{b:02x}")).collect();
                row([("name", NuVal::String(name)), ("type", NuVal::String(vector.ty)), ("hex", NuVal::String(hex))])
            })
            .collect(),
    )
}

/// The vectors' model.
pub const VECTORS_MODEL: &str = "tests/vectors/Vectors.nutype";
