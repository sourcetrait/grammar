//! A NuVal held to a NuType.
use crate::*;

impl NuType {
    /// Holds `value` to the type: a record or a table exactly its declared
    /// fields, an open one any, `path` and `directory` strings.
    pub fn check(&self, value: &NuVal) -> NuTypeResult<()> {
        let fits = match (self, value) {
            (Self::Any, _)
            | (Self::Nothing, NuVal::Nothing)
            | (Self::Bool, NuVal::Bool(_))
            | (Self::Int, NuVal::Int(_))
            | (Self::Float, NuVal::Float(_))
            | (Self::Number, NuVal::Int(_) | NuVal::Float(_))
            | (Self::String | Self::Path | Self::Directory, NuVal::String(_))
            | (Self::Glob, NuVal::Glob(_) | NuVal::String(_))
            | (Self::Binary, NuVal::Binary(_))
            | (Self::Date, NuVal::Date(_))
            | (Self::Duration, NuVal::Duration(_))
            | (Self::Filesize, NuVal::Filesize(_))
            | (Self::Range, NuVal::Range(_))
            | (Self::CellPath, NuVal::CellPath(_)) => true,
            (Self::List(element), NuVal::List(items)) => {
                for (index, item) in items.iter().enumerate() {
                    element.check(item).map_err(|e| e.within(index))?;
                }
                true
            }
            (Self::Record(fields), NuVal::Record(entries)) => {
                check_fields(fields, entries)?;
                true
            }
            (Self::Table(columns), NuVal::List(rows)) => {
                for (index, row) in rows.iter().enumerate() {
                    let NuVal::Record(entries) = row else {
                        return Err(mismatch("a record", row).within(index));
                    };
                    check_fields(columns, entries).map_err(|e| e.within(index))?;
                }
                true
            }
            (Self::OneOf(members), value) => {
                if members.iter().any(|member| member.check(value).is_ok()) {
                    true
                } else {
                    return Err(mismatch(&format!("one of `{self}`"), value));
                }
            }
            _ => false,
        };
        if fits { Ok(()) } else { Err(mismatch(&format!("`{self}`"), value)) }
    }
}

/// A record's entries held to the declared fields; none declared is open.
fn check_fields(fields: &[NuField], entries: &[(String, NuVal)]) -> NuTypeResult<()> {
    if fields.is_empty() {
        return Ok(());
    }
    for (key, _) in entries {
        if !fields.iter().any(|f| f.name == *key) {
            return Err(NuTypeError::Mismatch { path: key.clone(), reason: "an undeclared field".into() });
        }
    }
    for field in fields {
        match entries.iter().find(|(key, _)| *key == field.name) {
            Some((_, value)) => field.ty.check(value).map_err(|e| e.within(&field.name))?,
            None => {
                return Err(NuTypeError::Mismatch {
                    path: field.name.clone(),
                    reason: "a declared field is missing".into(),
                });
            }
        }
    }
    Ok(())
}

/// A value that isn't what was expected.
fn mismatch(expected: &str, value: &NuVal) -> NuTypeError {
    NuTypeError::Mismatch { path: String::new(), reason: format!("expected {expected}, found {}", kind(value)) }
}

/// A value's kind as nushell names its type.
fn kind(value: &NuVal) -> &'static str {
    match value {
        NuVal::Bool(_) => "bool",
        NuVal::Int(_) => "int",
        NuVal::Float(_) => "float",
        NuVal::String(_) => "string",
        NuVal::Glob(_) => "glob",
        NuVal::Filesize(_) => "filesize",
        NuVal::Duration(_) => "duration",
        NuVal::Date(_) => "datetime",
        NuVal::Range(_) => "range",
        NuVal::Record(_) => "record",
        NuVal::List(_) => "list",
        NuVal::Binary(_) => "binary",
        NuVal::CellPath(_) => "cell-path",
        NuVal::Nothing => "nothing",
    }
}
