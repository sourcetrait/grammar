//! A type as nushell's annotations spell it, on one line or laid out as the
//! repository's `.nutype` files are.
use crate::*;

impl NuType {
    /// The type on one line: `record<a: int, b: list<string>>`.
    pub fn to_inline(&self) -> String {
        match self {
            Self::List(element) => format!("list<{}>", element.to_inline()),
            Self::Record(fields) => format!("record<{}>", fields_inline(fields)),
            Self::Table(columns) => format!("table<{}>", fields_inline(columns)),
            Self::OneOf(members) => {
                let members: Vec<String> = members.iter().map(Self::to_inline).collect();
                format!("oneof<{}>", members.join(", "))
            }
            scalar => scalar.name().to_string(),
        }
    }

    /// The type laid out: a record's or a table's fields a line each, four
    /// spaces a level, and a list's or a union's members too where one holds
    /// such a record.
    pub fn to_block(&self) -> String {
        block(self, 0)
    }
}

impl NuTypeFile {
    /// The file's text: its documentation as `# ` lines, a blank line, and
    /// its type laid out.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for line in &self.doc {
            if line.is_empty() {
                out.push_str("#\n");
            } else {
                out.push_str(&format!("# {line}\n"));
            }
        }
        if !self.doc.is_empty() {
            out.push('\n');
        }
        out.push_str(&self.ty.to_block());
        out.push('\n');
        out
    }
}

/// Fields on one line: `a: int, b: string`.
fn fields_inline(fields: &[NuField]) -> String {
    let fields: Vec<String> =
        fields.iter().map(|f| format!("{}: {}", field_name(&f.name), f.ty.to_inline())).collect();
    fields.join(", ")
}

/// Whether the type takes lines of its own: a record or a table with
/// fields, or a list or a union holding one.
fn spans_lines(ty: &NuType) -> bool {
    match ty {
        NuType::Record(fields) | NuType::Table(fields) => !fields.is_empty(),
        NuType::List(element) => spans_lines(element),
        NuType::OneOf(members) => members.iter().any(spans_lines),
        _ => false,
    }
}

/// The type laid out at `depth`.
fn block(ty: &NuType, depth: usize) -> String {
    if !spans_lines(ty) {
        return ty.to_inline();
    }
    let inner = "    ".repeat(depth + 1);
    let outer = "    ".repeat(depth);
    let items: Vec<String> = match ty {
        NuType::Record(fields) | NuType::Table(fields) => fields
            .iter()
            .map(|f| format!("{inner}{}: {}", field_name(&f.name), block(&f.ty, depth + 1)))
            .collect(),
        NuType::List(element) => vec![format!("{inner}{}", block(element, depth + 1))],
        NuType::OneOf(members) => members.iter().map(|m| format!("{inner}{}", block(m, depth + 1))).collect(),
        _ => Vec::new(),
    };
    format!("{}<\n{}\n{outer}>", ty.name(), items.join(",\n"))
}

/// A field's name: bare where nushell reads it back as itself, else quoted.
fn field_name(name: &str) -> String {
    let bare = !name.is_empty()
        && !matches!(name, "true" | "false" | "null")
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '-'));
    if bare {
        return name.to_string();
    }
    let mut out = String::from("\"");
    for c in name.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}
