//! The contract's types, read from a signature, and values held to them.
use crate::*;

/// A type in the signature block's compact spelling, no spaces.
fn compact(ty: &NuType) -> std::string::String {
    match ty {
        NuType::Record(fields) => format!("record<{}>", compact_fields(fields)),
        NuType::Table(columns) => format!("table<{}>", compact_fields(columns)),
        NuType::OneOf(members) => {
            let parts: Vec<std::string::String> = members.iter().map(compact).collect();
            format!("oneof<{}>", parts.join(","))
        }
        NuType::List(element) => format!("list<{}>", compact(element)),
        scalar => scalar.name().to_string(),
    }
}

/// A record's fields in the compact spelling.
fn compact_fields(fields: &[NuField]) -> std::string::String {
    let parts: Vec<std::string::String> =
        fields.iter().map(|f| format!("{}:{}", f.name, compact(&f.ty))).collect();
    parts.join(",")
}

/// A signature group: a record as its fields, a table or nothing spelled whole.
pub fn signature_group(ty: &NuType) -> std::string::String {
    match ty {
        NuType::Record(fields) => format!("<{}>", compact_fields(fields)),
        other => format!("<{}>", compact(other)),
    }
}

/// Why an open record or table cannot type a definition's `data` at its top.
const OPEN_AT_TOP: &str = "an open `record<>` or `table<>` stands only nested; declare the fields";

/// Why an open record or table cannot type a return at any depth.
const OPEN_RETURNED: &str =
    "an open `record<>` or `table<>` is refused in a return type; declare the fields";

/// A contract's `data` type: `nothing`, or a typed record or table.
pub(crate) fn data_typedef(shape: &nu::SyntaxShape) -> Result<NuType, String> {
    match shape {
        nu::SyntaxShape::Nothing => Ok(NuType::Nothing),
        nu::SyntaxShape::Record(columns) | nu::SyntaxShape::Table(columns)
            if columns.is_empty() =>
        {
            Err(OPEN_AT_TOP.to_string())
        }
        nu::SyntaxShape::Record(_) | nu::SyntaxShape::Table(_) => {
            let ty = NuType::try_from(shape).map_err(not_data)?;
            grammar(&ty, None)?;
            Ok(ty)
        }
        other => Err(format!(
            "typed `nothing`, `record<...>`, or `table<...>`, not `{other}`"
        )),
    }
}

/// A contract's return type: `nothing`, or a typed record or table, no
/// record or table open at any depth.
pub(crate) fn result_typedef(ty: &nu::Type) -> Result<NuType, String> {
    match ty {
        nu::Type::Nothing => Ok(NuType::Nothing),
        nu::Type::Record(_) | nu::Type::Table(_) => {
            let typedef = NuType::try_from(ty).map_err(not_data)?;
            grammar(&typedef, Some(OPEN_RETURNED))?;
            Ok(typedef)
        }
        other => Err(format!("typed `nothing`, `record<...>`, or `table<...>`, not `{other}`")),
    }
}

/// A shape or type no data takes, in the contract's words.
fn not_data(error: NuTypeError) -> String {
    match error {
        NuTypeError::Unsupported { shape } => format!("`{shape}` is not a data type"),
        other => other.to_string(),
    }
}

/// The contract's grammar below the top: no `any`, no list of tables, and
/// no open record or table where `open` names why.
fn grammar(ty: &NuType, open: Option<&str>) -> Result<(), String> {
    match ty {
        NuType::Any => Err("`any` is not a grammar type".to_string()),
        NuType::Record(fields) | NuType::Table(fields) => {
            if let Some(why) = open
                && fields.is_empty()
            {
                return Err(why.to_string());
            }
            fields.iter().try_for_each(|f| grammar(&f.ty, open))
        }
        NuType::List(element) => {
            let of_tables = match element.as_ref() {
                NuType::Table(_) => true,
                NuType::List(inner) => matches!(inner.as_ref(), NuType::Record(_)),
                _ => false,
            };
            if of_tables {
                return Err("a list of tables is refused; hold each table in a record".to_string());
            }
            grammar(element, open)
        }
        NuType::OneOf(members) => members.iter().try_for_each(|m| grammar(m, open)),
        _ => Ok(()),
    }
}

/// Hold `value` to `typedef`: each record and table exactly its declared fields.
pub fn check_value(
    value: &nu::Value,
    typedef: &NuType,
    at: &str,
) -> Result<(), String> {
    match typedef {
        NuType::Record(fields) => {
            let Ok(record) = value.as_record() else {
                return Err(mismatch(value, "record", at));
            };
            check_fields(record, fields, at)
        }
        NuType::Table(columns) => {
            let Ok(rows) = value.as_list() else {
                return Err(mismatch(value, "table", at));
            };
            for (index, row) in rows.iter().enumerate() {
                let row_at = format!("{at}[{index}]");
                let Ok(record) = row.as_record() else {
                    return Err(mismatch(row, "record", &row_at));
                };
                check_fields(record, columns, &row_at)?;
            }
            Ok(())
        }
        NuType::List(element) => {
            let Ok(items) = value.as_list() else {
                return Err(mismatch(value, "list", at));
            };
            for (index, item) in items.iter().enumerate() {
                check_value(item, element, &format!("{at}[{index}]"))?;
            }
            Ok(())
        }
        NuType::OneOf(members) => {
            if members.iter().any(|member| check_value(value, member, at).is_ok()) {
                Ok(())
            } else {
                Err(format!(
                    "{at}: a value typed {} matches none of `{}`",
                    value.get_type(),
                    typedef.to_inline(),
                ))
            }
        }
        scalar => conform(scalar_conforms(value, scalar), value, scalar.name(), at),
    }
}

/// A record's fields held to the declared ones; none declared is an open record.
fn check_fields(
    record: &nu::Record,
    declared: &[NuField],
    at: &str,
) -> Result<(), String> {
    if declared.is_empty() {
        return Ok(());
    }
    for (key, _) in record.iter() {
        if !declared.iter().any(|field| field.name == *key) {
            return Err(format!("{at}.{key}: an undeclared field"));
        }
    }
    for field in declared {
        match record.get(&field.name) {
            Some(value) => check_value(value, &field.ty, &format!("{at}.{}", field.name))?,
            None => return Err(format!("{at}.{}: a declared field is missing", field.name)),
        }
    }
    Ok(())
}

/// Whether a value's variant is the scalar's; `path` and `directory` are strings.
fn scalar_conforms(
    value: &nu::Value,
    scalar: &NuType,
) -> bool {
    matches!(
        (scalar, value),
        (NuType::Any, _)
            | (NuType::Nothing, nu::Value::Nothing { .. })
            | (NuType::Int, nu::Value::Int { .. })
            | (NuType::Float, nu::Value::Float { .. })
            | (NuType::Number, nu::Value::Int { .. } | nu::Value::Float { .. })
            | (NuType::String | NuType::Path | NuType::Directory, nu::Value::String { .. })
            | (NuType::Glob, nu::Value::Glob { .. } | nu::Value::String { .. })
            | (NuType::Bool, nu::Value::Bool { .. })
            | (NuType::Date, nu::Value::Date { .. })
            | (NuType::Duration, nu::Value::Duration { .. })
            | (NuType::Filesize, nu::Value::Filesize { .. })
            | (NuType::Binary, nu::Value::Binary { .. })
            | (NuType::Range, nu::Value::Range { .. })
            | (NuType::CellPath, nu::Value::CellPath { .. })
    )
}

/// `Ok` when the value conforms, else the mismatch at its path.
fn conform(
    conforms: bool,
    value: &nu::Value,
    expected: &str,
    at: &str,
) -> Result<(), String> {
    if conforms {
        Ok(())
    } else {
        Err(mismatch(value, expected, at))
    }
}

/// A value's type against the one expected at its path.
fn mismatch(
    value: &nu::Value,
    expected: &str,
    at: &str,
) -> String {
    format!("{at}: expected {expected}, got {}", value.get_type())
}

/// A type in nushell's spelling, an open record or table as `record<>`.
pub fn typedef_text(typedef: &NuType) -> std::string::String {
    typedef.to_inline()
}
