//! NuVal as NUON text, written as nushell's `to nuon` writes it.
use crate::*;

/// How NUON text is laid out: the styles nushell's `to nuon` writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NuonStyle {
    /// One line, a space after each comma and colon: `{a: 1, b: [x, y]}`.
    #[default]
    Default,
    /// One line with no whitespace: `{a:1,b:[x,y]}`.
    Raw,
    /// A line an entry, indented by this many spaces a level.
    Spaces(usize),
    /// A line an entry, indented by this many tabs a level.
    Tabs(usize),
}

impl NuVal {
    /// The value as NUON text in `style`, as nushell's `to nuon` writes it.
    pub fn to_nuon(&self, style: NuonStyle) -> String {
        let indent = match style {
            NuonStyle::Default => None,
            NuonStyle::Raw => Some(String::new()),
            NuonStyle::Spaces(n) => Some(" ".repeat(n)),
            NuonStyle::Tabs(n) => Some("\t".repeat(n)),
        };
        text(self, 0, indent.as_deref())
    }
}

/// A value's text; a string is quoted, standing outside any structure.
fn text(value: &NuVal, depth: usize, indent: Option<&str>) -> String {
    let (nl, sep, kv_sep) = separators(indent);
    let idt = indentation(depth, indent);
    let idt_po = indentation(depth + 1, indent);
    match value {
        NuVal::Binary(bytes) => {
            let hex: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
            format!("0x[{hex}]")
        }
        NuVal::Bool(b) => b.to_string(),
        NuVal::CellPath(members) => cell_path(members),
        NuVal::Date(date) => rfc3339(date),
        NuVal::Duration(ns) => format!("{ns}ns"),
        NuVal::Filesize(bytes) => format!("{bytes}b"),
        NuVal::Float(f) => obvious_float(*f),
        NuVal::Glob(glob) => quote(&glob.glob),
        NuVal::Int(i) => i.to_string(),
        NuVal::List(items) => list(items, depth, indent),
        NuVal::Nothing => "null".to_string(),
        NuVal::Range(range) => range_text(range),
        NuVal::Record(entries) => {
            let entries: Vec<String> = entries
                .iter()
                .map(|(col, val)| format!("{idt_po}{}:{kv_sep}{}", key(col), nested(val, depth + 1, indent)))
                .collect();
            format!("{{{nl}{}{nl}{idt}}}", entries.join(&format!(",{sep}{nl}")))
        }
        NuVal::String(s) => quote(s),
    }
}

/// A value inside a structure, where a string goes bare unless it can't.
fn nested(value: &NuVal, depth: usize, indent: Option<&str>) -> String {
    match value {
        NuVal::String(s) => key(s),
        other => text(other, depth, indent),
    }
}

/// A list, or a table where every item is a record of the same columns.
fn list(items: &[NuVal], depth: usize, indent: Option<&str>) -> String {
    let (nl, sep, _) = separators(indent);
    let idt = indentation(depth, indent);
    let idt_po = indentation(depth + 1, indent);
    let idt_pt = indentation(depth + 2, indent);
    let headers = columns(items);
    let is_table = !headers.is_empty()
        && items.iter().all(|item| {
            matches!(item, NuVal::Record(entries) if entries.iter().map(|(k, _)| k).eq(headers.iter().copied()))
        });
    if !is_table {
        let items: Vec<String> =
            items.iter().map(|item| format!("{idt_po}{}", nested(item, depth + 1, indent))).collect();
        return format!("[{nl}{}{nl}{idt}]", items.join(&format!(",{sep}{nl}")));
    }
    let rows: Vec<Vec<String>> = items
        .iter()
        .filter_map(|item| match item {
            NuVal::Record(entries) => {
                Some(entries.iter().map(|(_, val)| nested(val, depth + 2, indent)).collect())
            }
            _ => None,
        })
        .collect();
    if indent.is_some_and(|i| !i.is_empty()) {
        let names: Vec<String> = headers.iter().map(|h| key(h)).collect();
        let widths: Vec<usize> = (0..names.len())
            .map(|i| rows.iter().map(|row| row[i].len()).fold(names[i].len(), usize::max))
            .collect();
        let pad_row = |cells: &[String]| -> String {
            let mut out = String::new();
            for (i, cell) in cells.iter().enumerate() {
                if i < cells.len() - 1 {
                    out.push_str(&format!("{:<width$}", format!("{cell},"), width = widths[i] + 2));
                } else {
                    out.push_str(cell);
                }
            }
            out
        };
        let header_row = format!("[{}];", pad_row(&names));
        let value_rows: Vec<String> = rows.iter().map(|row| format!("[{}]", pad_row(row))).collect();
        let value_rows = value_rows.join(&format!(",{nl}{idt_po}"));
        format!("[{nl}{idt_po}{header_row}{sep}{nl}{idt_po}{value_rows}{nl}{idt}]")
    } else {
        let headers: Vec<String> = headers.iter().map(|h| format!("{idt}{}", key(h))).collect();
        let headers = headers.join(&format!(",{sep}{nl}{idt_pt}"));
        let rows: Vec<String> = rows.iter().map(|row| row.join(&format!(",{sep}{nl}{idt_pt}"))).collect();
        let rows = rows.join(&format!("{nl}{idt_po}],{sep}{nl}{idt_po}[{nl}{idt_pt}"));
        format!(
            "[{nl}{idt_po}[{nl}{idt_pt}{headers}{nl}{idt_po}];{sep}{nl}{idt_po}[{nl}{idt_pt}{rows}{nl}{idt_po}]{nl}{idt}]"
        )
    }
}

/// Every record's columns in order of first appearance, none where an item
/// isn't a record.
fn columns(items: &[NuVal]) -> Vec<&String> {
    let mut columns: Vec<&String> = Vec::new();
    for item in items {
        let NuVal::Record(entries) = item else {
            return Vec::new();
        };
        for (col, _) in entries {
            if !columns.contains(&col) {
                columns.push(col);
            }
        }
    }
    columns
}

/// The newline, the inline separator, and the key-value separator.
fn separators(indent: Option<&str>) -> (&'static str, &'static str, &'static str) {
    match indent {
        Some("") => ("", "", ""),
        Some(_) => ("\n", "", " "),
        None => ("", " ", " "),
    }
}

/// The indentation at `depth`.
fn indentation(depth: usize, indent: Option<&str>) -> String {
    indent.map_or_else(String::new, |i| i.repeat(depth))
}

/// A record key, a column, or a nested string: bare unless it can't be.
pub(crate) fn key(s: &str) -> String {
    if needs_quoting(s) { quote(s) } else { s.to_string() }
}

/// A string in double quotes, its quotes and backslashes escaped.
pub(crate) fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Whether a bare string would read back as something else, nushell's rule.
pub(crate) fn needs_quoting(s: &str) -> bool {
    if s.is_empty() || matches!(s, "true" | "false" | "null" | "&&") {
        return true;
    }
    let special = |c: char| {
        matches!(
            c,
            '[' | ']' | ':' | '`' | '{' | '}' | '#' | '\'' | '"' | ';' | '(' | ')' | '|' | '$' | ',' | '.'
                | '!' | '?' | '='
        )
    };
    if s.chars().any(|c| special(c) || c.is_whitespace() || is_decimal_digit(c)) {
        return true;
    }
    let unsigned = s.strip_prefix(['+', '-']).unwrap_or(s);
    ["inf", "infinity", "nan"].iter().any(|word| unsigned.eq_ignore_ascii_case(word))
}

/// The first and last of each run of Unicode's decimal digits, the `\d` of
/// the regex nushell's quoting rule uses.
const DECIMAL_DIGITS: [(u32, u32); 71] = [
    (0x30, 0x39), (0x660, 0x669), (0x6f0, 0x6f9), (0x7c0, 0x7c9), (0x966, 0x96f),
    (0x9e6, 0x9ef), (0xa66, 0xa6f), (0xae6, 0xaef), (0xb66, 0xb6f), (0xbe6, 0xbef),
    (0xc66, 0xc6f), (0xce6, 0xcef), (0xd66, 0xd6f), (0xde6, 0xdef), (0xe50, 0xe59),
    (0xed0, 0xed9), (0xf20, 0xf29), (0x1040, 0x1049), (0x1090, 0x1099), (0x17e0, 0x17e9),
    (0x1810, 0x1819), (0x1946, 0x194f), (0x19d0, 0x19d9), (0x1a80, 0x1a89), (0x1a90, 0x1a99),
    (0x1b50, 0x1b59), (0x1bb0, 0x1bb9), (0x1c40, 0x1c49), (0x1c50, 0x1c59), (0xa620, 0xa629),
    (0xa8d0, 0xa8d9), (0xa900, 0xa909), (0xa9d0, 0xa9d9), (0xa9f0, 0xa9f9), (0xaa50, 0xaa59),
    (0xabf0, 0xabf9), (0xff10, 0xff19), (0x104a0, 0x104a9), (0x10d30, 0x10d39), (0x10d40, 0x10d49),
    (0x11066, 0x1106f), (0x110f0, 0x110f9), (0x11136, 0x1113f), (0x111d0, 0x111d9), (0x112f0, 0x112f9),
    (0x11450, 0x11459), (0x114d0, 0x114d9), (0x11650, 0x11659), (0x116c0, 0x116c9), (0x116d0, 0x116e3),
    (0x11730, 0x11739), (0x118e0, 0x118e9), (0x11950, 0x11959), (0x11bf0, 0x11bf9), (0x11c50, 0x11c59),
    (0x11d50, 0x11d59), (0x11da0, 0x11da9), (0x11f50, 0x11f59), (0x16130, 0x16139), (0x16a60, 0x16a69),
    (0x16ac0, 0x16ac9), (0x16b50, 0x16b59), (0x16d70, 0x16d79), (0x1ccf0, 0x1ccf9), (0x1d7ce, 0x1d7ff),
    (0x1e140, 0x1e149), (0x1e2f0, 0x1e2f9), (0x1e4f0, 0x1e4f9), (0x1e5f1, 0x1e5fa), (0x1e950, 0x1e959),
    (0x1fbf0, 0x1fbf9),
];

/// Whether `c` is one of Unicode's decimal digits.
fn is_decimal_digit(c: char) -> bool {
    let c = u32::from(c);
    DECIMAL_DIGITS.iter().any(|&(first, last)| (first..=last).contains(&c))
}

/// A float with a decimal point even when it is whole: `1.0`, `1.5`.
pub(crate) fn obvious_float(f: f64) -> String {
    if f.fract() == 0.0 { format!("{f:.1}") } else { f.to_string() }
}

/// A cell path: `$.a?.0."b c"!`, or `$.` for none.
fn cell_path(members: &[CellPathMemberData]) -> String {
    let mut out = String::from("$");
    for member in members {
        out.push('.');
        match member {
            CellPathMemberData::Int { value, optional } => {
                out.push_str(&value.to_string());
                if *optional {
                    out.push('?');
                }
            }
            CellPathMemberData::String { value, optional, case_sensitive } => {
                out.push_str(&key(value));
                if !case_sensitive {
                    out.push('!');
                }
                if *optional {
                    out.push('?');
                }
            }
        }
    }
    if members.is_empty() {
        out.push('.');
    }
    out
}

/// A range: its start, the next value where the step isn't one, its end.
fn range_text(range: &RangeData) -> String {
    match range {
        RangeData::Int(r) => bounds(
            r.start.to_string(),
            (r.step != 1).then(|| r.start.wrapping_add(r.step).to_string()),
            &r.end,
            |end| end.to_string(),
        ),
        RangeData::Float(r) => bounds(
            obvious_float(r.start),
            (r.step != 1.0).then(|| obvious_float(r.start + r.step)),
            &r.end,
            |end| obvious_float(*end),
        ),
    }
}

/// `start..`, then `next..` where given, then the end's bound.
fn bounds<T>(start: String, next: Option<String>, end: &Bounded<T>, text: impl Fn(&T) -> String) -> String {
    let mut out = format!("{start}..");
    if let Some(next) = next {
        out.push_str(&next);
        out.push_str("..");
    }
    match end {
        Bounded::Included(end) => out.push_str(&text(end)),
        Bounded::Excluded(end) => {
            out.push('<');
            out.push_str(&text(end));
        }
        Bounded::Unbounded => {}
    }
    out
}

/// A date as chrono's RFC 3339 writes it: its fraction in 0, 3, 6, or 9
/// digits, its offset to the minute.
pub(crate) fn rfc3339(date: &DateData) -> String {
    let DateData { seconds, seconds_fraction, utc_offset_minus } = *date;
    let local = seconds + i64::from(utc_offset_minus);
    let (year, month, day) = civil(local.div_euclid(86_400));
    let of_day = local.rem_euclid(86_400);
    let (hour, minute, mut second) = (of_day / 3600, of_day % 3600 / 60, of_day % 60);
    let mut nano = seconds_fraction;
    if nano >= 1_000_000_000 {
        second += 1;
        nano -= 1_000_000_000;
    }
    let mut out = if (0..=9999).contains(&year) { format!("{year:04}") } else { format!("{year:+05}") };
    out.push_str(&format!("-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}"));
    if nano != 0 {
        let fraction = if nano % 1_000_000 == 0 {
            format!(".{:03}", nano / 1_000_000)
        } else if nano % 1_000 == 0 {
            format!(".{:06}", nano / 1_000)
        } else {
            format!(".{nano:09}")
        };
        out.push_str(&fraction);
    }
    let (sign, offset) = if utc_offset_minus < 0 { ('-', -utc_offset_minus) } else { ('+', utc_offset_minus) };
    let minutes = (offset + 30) / 60;
    out.push_str(&format!("{sign}{:02}:{:02}", minutes / 60, minutes % 60));
    out
}

/// The proleptic Gregorian year, month, and day `days` after 1970-01-01.
pub(crate) fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}
