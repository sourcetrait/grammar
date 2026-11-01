//! NUON text read into a NuVal, as nushell's `from nuon` reads it.
use crate::*;

impl NuVal {
    /// The value NUON text holds, read as nushell's `from nuon` reads it.
    pub fn from_nuon(text: &str) -> NuonResult<NuVal> {
        let mut lexer = Lexer::new(text, 0..text.len());
        let mut items = Vec::new();
        while let Some(token) = lexer.next(&[], &[])? {
            match token {
                Token::Item(span) => items.push(span),
                Token::Semicolon(at) => return Err(read_error(at, "a semicolon starts a second value")),
                Token::Special(at, _) => return Err(read_error(at, "a stray separator")),
            }
        }
        match items.as_slice() {
            [] => Ok(NuVal::Nothing),
            [span] => top(text, span.clone()),
            [_, second, ..] => Err(read_error(second.start, "NUON holds one value")),
        }
    }
}

/// The bytes a list, a record, or binary separates items with beside
/// whitespace.
const SEPARATORS: &[u8] = b"\n\r,";

/// A read error at `offset`.
fn read_error(offset: usize, reason: impl Into<String>) -> NuonError {
    NuonError::Read { offset, reason: reason.into() }
}

/// A token: a value's text, a semicolon, or a context's special byte.
enum Token {
    Item(Range<usize>),
    Semicolon(usize),
    Special(usize, u8),
}

/// nushell's lexer over a span: comments and whitespace skipped, brackets
/// and quotes holding a token together.
struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
    end: usize,
}

impl<'a> Lexer<'a> {
    fn new(text: &'a str, span: Range<usize>) -> Self {
        Self { bytes: text.as_bytes(), pos: span.start, end: span.end }
    }

    /// The next token, `whitespace` and `special` the context's own bytes.
    fn next(&mut self, whitespace: &[u8], special: &[u8]) -> NuonResult<Option<Token>> {
        while self.pos < self.end {
            let c = self.bytes[self.pos];
            if matches!(c, b' ' | b'\t' | b'\n' | b'\r') || whitespace.contains(&c) {
                self.pos += 1;
            } else if c == b'#' {
                while self.pos < self.end && !matches!(self.bytes[self.pos], b'\n' | b'\r') {
                    self.pos += 1;
                }
            } else if c == b';' {
                self.pos += 1;
                return Ok(Some(Token::Semicolon(self.pos - 1)));
            } else if c == b'|' {
                return Err(read_error(self.pos, "a pipe in NUON"));
            } else if special.contains(&c) {
                self.pos += 1;
                return Ok(Some(Token::Special(self.pos - 1, c)));
            } else {
                let start = self.pos;
                self.pos = self.item_end(start, whitespace, special)?;
                return Ok(Some(Token::Item(start..self.pos)));
            }
        }
        Ok(None)
    }

    /// Where the item at `start` ends: the first terminator outside any
    /// bracket, quote, or raw string.
    fn item_end(&self, start: usize, whitespace: &[u8], special: &[u8]) -> NuonResult<usize> {
        let mut closers: Vec<(u8, usize)> = Vec::new();
        let mut i = start;
        let mut previous: Option<u8> = None;
        while i < self.end {
            let c = self.bytes[i];
            let fresh = previous.is_none_or(|p| p.is_ascii_whitespace() || b"[{(,:".contains(&p));
            if c == b'r' && fresh && self.bytes.get(i + 1) == Some(&b'#') {
                i = self.raw_end(i)?;
                previous = Some(b'#');
                continue;
            }
            match c {
                b'"' | b'\'' | b'`' => {
                    i = self.quote_end(i)?;
                    previous = Some(c);
                    continue;
                }
                b'#' if !closers.is_empty() && previous.is_none_or(|p| p.is_ascii_whitespace()) => {
                    while i < self.end && !matches!(self.bytes[i], b'\n' | b'\r') {
                        i += 1;
                    }
                    previous = Some(b'\n');
                    continue;
                }
                b'[' => closers.push((b']', i)),
                b'{' => closers.push((b'}', i)),
                b'(' => closers.push((b')', i)),
                b']' | b'}' | b')' => match closers.last() {
                    Some(&(closer, _)) if closer == c => {
                        closers.pop();
                    }
                    _ => return Err(read_error(i, format!("an unbalanced `{}`", c as char))),
                },
                _ if closers.is_empty()
                    && (matches!(c, b' ' | b'\t' | b'\n' | b'\r' | b'|' | b';')
                        || whitespace.contains(&c)
                        || special.contains(&c)) =>
                {
                    break;
                }
                _ => {}
            }
            previous = Some(c);
            i += 1;
        }
        match closers.last() {
            Some(&(closer, at)) => Err(read_error(at, format!("no `{}` closes this", closer as char))),
            None => Ok(i),
        }
    }

    /// The end of the quoted string opening at `start`; only a double quote
    /// escapes.
    fn quote_end(&self, start: usize) -> NuonResult<usize> {
        let quote = self.bytes[start];
        let mut i = start + 1;
        while i < self.end {
            match self.bytes[i] {
                b'\\' if quote == b'"' => i += 2,
                c if c == quote => return Ok(i + 1),
                _ => i += 1,
            }
        }
        Err(read_error(start, "an unclosed quote"))
    }

    /// The end of the raw string `r#'...'#` opening at `start`.
    fn raw_end(&self, start: usize) -> NuonResult<usize> {
        let hashes = self.bytes[start + 1..self.end].iter().take_while(|b| **b == b'#').count();
        let open = start + 1 + hashes;
        if self.bytes.get(open) != Some(&b'\'') {
            return Err(read_error(start, "a raw string opens with r#'"));
        }
        let mut i = open + 1;
        while i < self.end {
            if self.bytes[i] == b'\''
                && self.bytes.len() >= i + 1 + hashes
                && self.bytes[i + 1..i + 1 + hashes].iter().all(|b| *b == b'#')
            {
                return Ok(i + 1 + hashes);
            }
            i += 1;
        }
        Err(read_error(start, "an unclosed raw string"))
    }
}

/// A shape's read: the value, a miss that lets the next shape try, or a
/// failure that ends the read.
enum Missed {
    Miss,
    Fail(NuonError),
}

/// A shape's result.
type Shape<T> = Result<T, Missed>;

/// A failure at `offset`, ending the read.
fn fail<T>(offset: usize, reason: impl Into<String>) -> Shape<T> {
    Err(Missed::Fail(read_error(offset, reason)))
}

/// The document's one value, which nushell reads only where it isn't a
/// command: a quoted string, a literal, or a structure.
fn top(text: &str, span: Range<usize>) -> NuonResult<NuVal> {
    let token = &text[span.clone()];
    if matches!(token, "not" | "if" | "match") {
        return Err(read_error(span.start, format!("`{token}` isn't a value")));
    }
    let value_like = matches!(token, "true" | "false" | "null")
        || token.starts_with("r#")
        || token.starts_with(['(', '{', '[', '$', '"', '\'', '-'])
        || number(token).is_ok()
        || filesize(token, span.start).is_ok()
        || duration(token, span.start).is_ok()
        || datetime(token).is_ok()
        || !matches!(binary(token, span.start), Err(Missed::Miss))
        || range(token, span.start).is_ok();
    if !value_like {
        return Err(read_error(span.start, "a bare word reads as a command; quote it"));
    }
    value(text, span)
}

/// A value of any shape, as nushell's parser tries them in turn.
fn value(text: &str, span: Range<usize>) -> NuonResult<NuVal> {
    let token = &text[span.clone()];
    let at = span.start;
    match token.as_bytes()[0] {
        b'$' => return cell_path(token, at),
        b'(' => return Err(read_error(at, "subexpressions aren't NUON")),
        b'{' => return record(text, span),
        b'[' => return list(text, span),
        b'r' if token.starts_with("r#") => return raw_string(token, at).map(NuVal::String),
        _ => {}
    }
    match token {
        "null" => return Ok(NuVal::Nothing),
        "true" => return Ok(NuVal::Bool(true)),
        "false" => return Ok(NuVal::Bool(false)),
        _ => {}
    }
    let shapes: [&dyn Fn() -> Shape<NuVal>; 8] = [
        &|| binary(token, at).map(NuVal::Binary),
        &|| range(token, at).map(NuVal::Range),
        &|| filesize(token, at).map(NuVal::Filesize),
        &|| duration(token, at).map(NuVal::Duration),
        &|| datetime(token).map(NuVal::Date),
        &|| int(token, at).map(NuVal::Int),
        &|| number(token),
        &|| string(token, at).map(NuVal::String),
    ];
    for shape in shapes {
        match shape() {
            Ok(value) => return Ok(value),
            Err(Missed::Miss) => {}
            Err(Missed::Fail(e)) => return Err(e),
        }
    }
    Err(read_error(at, "a value of no shape"))
}

/// A record: `key: value` pairs, commas and newlines between them.
fn record(text: &str, span: Range<usize>) -> NuonResult<NuVal> {
    let mut lexer = Lexer::new(text, span.start + 1..span.end - 1);
    let mut entries: Vec<(String, NuVal)> = Vec::new();
    while let Some(key) = lexer.next(SEPARATORS, b":")? {
        let Token::Item(key) = key else {
            return Err(read_error(token_at(&key), "a record key belongs here"));
        };
        if text[key.clone()].starts_with("...") {
            return Err(read_error(key.start, "the spread operator isn't NUON"));
        }
        match lexer.next(SEPARATORS, b":")? {
            Some(Token::Special(_, b':')) => {}
            Some(other) => return Err(read_error(token_at(&other), "a record key takes a `:`")),
            None => return Err(read_error(key.end, "a record key takes a `:`")),
        }
        let field = match lexer.next(SEPARATORS, &[])? {
            Some(Token::Item(field)) => field,
            Some(other) => return Err(read_error(token_at(&other), "a record field takes a value")),
            None => return Err(read_error(key.end, "a record field takes a value")),
        };
        let name = record_key(text, key.clone())?;
        if entries.iter().any(|(k, _)| *k == name) {
            return Err(read_error(key.start, format!("the column `{name}` appears twice")));
        }
        let read = value(text, field.clone())?;
        let token = &text[field.clone()];
        if let NuVal::String(_) = read
            && !token.starts_with(['"', '\'', '`'])
            && !token.starts_with("r#")
            && let Some(colon) = token.find(':')
        {
            return Err(read_error(field.start + colon, "a colon in a bare word specifying a record value"));
        }
        entries.push((name, read));
    }
    Ok(NuVal::Record(entries))
}

/// Where a token starts.
fn token_at(token: &Token) -> usize {
    match token {
        Token::Item(span) => span.start,
        Token::Semicolon(at) | Token::Special(at, _) => *at,
    }
}

/// A record key or a table column: a string, bare or quoted, never a raw
/// string, a variable, or a keyword.
fn record_key(text: &str, span: Range<usize>) -> NuonResult<String> {
    let token = &text[span.clone()];
    if token.starts_with(['$', '(', '{']) || token.starts_with("r#") || matches!(token, "true" | "false" | "null") {
        return Err(read_error(span.start, "only strings can be keys"));
    }
    string(token, span.start).map_err(|missed| match missed {
        Missed::Fail(e) => e,
        Missed::Miss => read_error(span.start, "only strings can be keys"),
    })
}

/// A list, or a table: `[[columns]; [row], ...]`.
fn list(text: &str, span: Range<usize>) -> NuonResult<NuVal> {
    let mut lexer = Lexer::new(text, span.start + 1..span.end - 1);
    let mut tokens = Vec::new();
    while let Some(token) = lexer.next(SEPARATORS, &[])? {
        tokens.push(token);
    }
    if let [Token::Item(head), Token::Semicolon(_), rows @ ..] = tokens.as_slice()
        && text[head.clone()].starts_with('[')
    {
        return table(text, head.clone(), rows, span.end - 1);
    }
    let mut items = Vec::with_capacity(tokens.len());
    for token in tokens {
        match token {
            Token::Item(item) if text[item.clone()].starts_with("...") => {
                return Err(read_error(item.start, "the spread operator isn't NUON"));
            }
            Token::Item(item) => items.push(value(text, item)?),
            other => return Err(read_error(token_at(&other), "a semicolon in a list")),
        }
    }
    Ok(NuVal::List(items))
}

/// A table's records, each row's cells under the header's columns.
fn table(text: &str, head: Range<usize>, rows: &[Token], end: usize) -> NuonResult<NuVal> {
    let NuVal::List(columns) = list(text, head.clone())? else {
        return Err(read_error(head.start, "a table's header is a list"));
    };
    let mut names: Vec<String> = Vec::with_capacity(columns.len());
    for (column, cell) in columns.into_iter().zip(cell_starts(text, head.clone())) {
        match column {
            NuVal::String(name) if !text[cell..].starts_with("r#") => {
                if names.contains(&name) {
                    return Err(read_error(cell, format!("the column `{name}` appears twice")));
                }
                names.push(name);
            }
            _ => return Err(read_error(cell, "only strings can be keys")),
        }
    }
    if rows.is_empty() {
        return Err(read_error(end, "a table holds a row"));
    }
    let mut records = Vec::with_capacity(rows.len());
    for row in rows {
        let Token::Item(row) = row else {
            return Err(read_error(token_at(row), "a table's items are lists"));
        };
        if !text[row.clone()].starts_with('[') {
            return Err(read_error(row.start, "a table's items are lists"));
        }
        let NuVal::List(cells) = list(text, row.clone())? else {
            return Err(read_error(row.start, "a table's items are lists"));
        };
        if cells.len() != names.len() {
            return Err(read_error(row.start, format!("a row of {} cells under {} columns", cells.len(), names.len())));
        }
        records.push(NuVal::Record(names.iter().cloned().zip(cells).collect()));
    }
    Ok(NuVal::List(records))
}

/// Where each of a list's items starts, for errors about one of them.
fn cell_starts(text: &str, span: Range<usize>) -> Vec<usize> {
    let mut lexer = Lexer::new(text, span.start + 1..span.end - 1);
    let mut starts = Vec::new();
    while let Ok(Some(token)) = lexer.next(SEPARATORS, &[]) {
        starts.push(token_at(&token));
    }
    starts
}

/// A string: double-quoted with escapes, single-quoted or backticked as it
/// stands, or bare.
fn string(token: &str, at: usize) -> Shape<String> {
    let bytes = token.as_bytes();
    if !matches!(bytes[0], b'\'' | b'"' | b'`') && token.contains('(') {
        return fail(at, "string interpolation isn't NUON");
    }
    for quote in *b"\"'" {
        if bytes[0] == quote {
            let last = bytes.iter().rposition(|b| *b == quote).unwrap_or(0);
            if last != bytes.len() - 1 && last != 0 {
                return fail(at + last + 1, "text after a closing quote");
            }
        }
    }
    let quoted = |q: u8| bytes.len() > 1 && bytes[0] == q && bytes[bytes.len() - 1] == q;
    if bytes[0] == b'"' {
        let inner = if quoted(b'"') { &token[1..token.len() - 1] } else { token };
        return unescape(inner, at + 1).map_err(Missed::Fail);
    }
    if quoted(b'\'') || quoted(b'`') {
        return Ok(token[1..token.len() - 1].to_string());
    }
    Ok(token.to_string())
}

/// A double-quoted string's content with its escapes resolved, byte by byte
/// as nushell resolves them, so hex escapes can spell UTF-8.
fn unescape(inner: &str, at: usize) -> NuonResult<String> {
    if !inner.contains('\\') {
        return Ok(inner.to_string());
    }
    let bytes = inner.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        let escape = at + i;
        let Some(&next) = bytes.get(i + 1) else {
            return Err(read_error(escape, "an escape with nothing after it"));
        };
        i += 2;
        match next {
            b'"' | b'\'' | b'\\' | b'/' | b'(' | b')' | b'{' | b'}' | b'$' | b'^' | b'#' | b'|' | b'~' | b' ' => {
                out.push(next);
            }
            b'a' => out.push(0x07),
            b'b' => out.push(0x08),
            b'e' => out.push(0x1b),
            b'f' => out.push(0x0c),
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'0' => out.push(0),
            b'x' => {
                let byte = bytes
                    .get(i..i + 2)
                    .and_then(|digits| std::str::from_utf8(digits).ok())
                    .filter(|digits| digits.bytes().all(|d| d.is_ascii_hexdigit()))
                    .and_then(|digits| u8::from_str_radix(digits, 16).ok())
                    .ok_or_else(|| read_error(escape, "a hex escape takes two hex digits"))?;
                out.push(byte);
                i += 2;
            }
            b'u' => {
                let (c, close) = bytes[i..]
                    .strip_prefix(b"{")
                    .and_then(|rest| rest.iter().position(|b| *b == b'}').map(|close| (&rest[..close], close)))
                    .filter(|(digits, _)| (1..=6).contains(&digits.len()))
                    .and_then(|(digits, close)| {
                        let code = u32::from_str_radix(std::str::from_utf8(digits).ok()?, 16).ok()?;
                        Some((char::from_u32(code)?, close))
                    })
                    .ok_or_else(|| read_error(escape, "a unicode escape is 1 to 6 hex digits in braces"))?;
                out.extend_from_slice(c.encode_utf8(&mut [0u8; 4]).as_bytes());
                i += close + 2;
            }
            other => return Err(read_error(escape, format!("an unknown escape `\\{}`", other as char))),
        }
    }
    String::from_utf8(out).map_err(|_| read_error(at, "escapes that don't spell UTF-8"))
}

/// A raw string's content: `r#'...'#`, as many hashes closing as opening.
fn raw_string(token: &str, at: usize) -> NuonResult<String> {
    let hashes = token[1..].bytes().take_while(|b| *b == b'#').count();
    let bytes = token.as_bytes();
    if bytes.len() < 2 * hashes + 3
        || bytes[1 + hashes] != b'\''
        || bytes[bytes.len() - hashes - 1] != b'\''
        || !bytes[bytes.len() - hashes..].iter().all(|b| *b == b'#')
    {
        return Err(read_error(at, "a raw string closes as it opens"));
    }
    Ok(token[2 + hashes..token.len() - 1 - hashes].to_string())
}

/// Binary: `0x[..]`, `0o[..]`, or `0b[..]`, digits padded to whole bytes.
fn binary(token: &str, at: usize) -> Shape<Vec<u8>> {
    let (base, per_byte, inner) = if let Some(inner) = token.strip_prefix("0x[") {
        (16, 2, inner)
    } else if let Some(inner) = token.strip_prefix("0o[") {
        (8, 3, inner)
    } else if let Some(inner) = token.strip_prefix("0b[") {
        (2, 8, inner)
    } else {
        return Err(Missed::Miss);
    };
    let Some(inner) = inner.strip_suffix(']') else {
        return Err(Missed::Miss);
    };
    let mut lexer = Lexer::new(token, 3..3 + inner.len());
    let mut digits = String::new();
    while let Some(item) = lexer.next(SEPARATORS, &[]).map_err(Missed::Fail)? {
        if let Token::Item(span) = item {
            digits.push_str(&token[span]);
        }
    }
    let pad = (per_byte - digits.chars().count() % per_byte) % per_byte;
    let digits: Vec<char> = std::iter::repeat_n('0', pad).chain(digits.chars()).collect();
    digits
        .chunks(per_byte)
        .map(|chunk| u8::from_str_radix(&chunk.iter().collect::<String>(), base))
        .collect::<Result<Vec<u8>, _>>()
        .or_else(|_| fail(at, format!("binary digits that aren't base {base}")))
}

/// An int: decimal, or `0x`, `0o`, `0b` read as u64 and cast, `_` dropped.
fn int(token: &str, at: usize) -> Shape<i64> {
    let token = token.replace('_', "");
    let prefixed = |radix: u32, digits: &str| match u64::from_str_radix(digits, radix) {
        Ok(n) => Ok(n as i64),
        Err(_) => fail(at, format!("digits that aren't radix {radix}")),
    };
    if token.is_empty() {
        Err(Missed::Miss)
    } else if let Some(digits) = token.strip_prefix("0b") {
        prefixed(2, digits)
    } else if let Some(digits) = token.strip_prefix("0o") {
        prefixed(8, digits)
    } else if let Some(digits) = token.strip_prefix("0x") {
        prefixed(16, digits)
    } else {
        token.parse::<i64>().map_err(|_| Missed::Miss)
    }
}

/// A number: an int, else a float, `_` dropped.
fn number(token: &str) -> Shape<NuVal> {
    if let Ok(i) = int(token, 0) {
        return Ok(NuVal::Int(i));
    }
    token.replace('_', "").parse::<f64>().map(NuVal::Float).map_err(|_| Missed::Miss)
}

/// A range: `from..to`, `from..next..to`, `..=` and `..<`, one bound at
/// least, each bound a number.
fn range(token: &str, at: usize) -> Shape<RangeData> {
    if token.starts_with("...") || !token.contains("..") {
        return Err(Missed::Miss);
    }
    let dots: Vec<usize> = token.match_indices("..").map(|(pos, _)| pos).collect();
    let (next_at, op_at) = match dots.as_slice() {
        [op] => (None, *op),
        [next, op] => (Some(*next), *op),
        _ => return Err(Missed::Miss),
    };
    let (exclusive, op) = match token.find("..<") {
        Some(pos) if pos == op_at => (true, "..<"),
        Some(_) => return Err(Missed::Miss),
        None if token[op_at..].starts_with("..=") => (false, "..="),
        None => (false, ".."),
    };
    let bound = |text: &str| number(text).map_err(|_| Missed::Miss);
    let from = if token.starts_with("..") { None } else { Some(bound(&token[..dots[0]])?) };
    let to = if token.ends_with(op) { None } else { Some(bound(&token[op_at + op.len()..])?) };
    if from.is_none() && to.is_none() {
        return Err(Missed::Miss);
    }
    let next = match next_at {
        Some(next) => Some(bound(&token[next + 2..op_at])?),
        None => None,
    };
    make_range(from, next, to, exclusive).or_else(|reason| fail(at, reason))
}

/// The range nushell's `Range::new` makes of the bounds: ints alone an int
/// range, any float a float range.
fn make_range(from: Option<NuVal>, next: Option<NuVal>, to: Option<NuVal>, exclusive: bool) -> Result<RangeData, String> {
    let is_float = |v: &Option<NuVal>| matches!(v, Some(NuVal::Float(_)));
    if is_float(&from) || is_float(&next) || is_float(&to) {
        let float = |v: Option<NuVal>| match v {
            Some(NuVal::Float(f)) => Some(f),
            Some(NuVal::Int(i)) => Some(i as f64),
            _ => None,
        };
        let (start, next, end) = (float(from).unwrap_or(0.0), float(next), float(to));
        if !start.is_finite() || end.is_some_and(f64::is_nan) || next.is_some_and(|n| n == start || !n.is_finite()) {
            return Err("a range nushell can't make".into());
        }
        let step = match (next, end) {
            (Some(next), Some(end)) if (next < start) != (end < start) => {
                return Err("a range stepping away from its end".into());
            }
            (Some(next), _) => next - start,
            (None, Some(end)) => {
                let diff = end - start;
                if diff == 0.0 {
                    return Err("a range with no length".into());
                }
                if diff.abs() < 1.0 {
                    diff.signum() * 10.0_f64.powf(diff.abs().log10().floor())
                } else if diff > 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
            (None, None) => 1.0,
        };
        let end = match end {
            Some(end) if end.is_infinite() => Bounded::Unbounded,
            Some(end) if exclusive => Bounded::Excluded(end),
            Some(end) => Bounded::Included(end),
            None => Bounded::Unbounded,
        };
        return Ok(RangeData::Float(TypedRangeData { start, step, end }));
    }
    let int = |v: Option<NuVal>| match v {
        Some(NuVal::Int(i)) => Some(i),
        _ => None,
    };
    let (start, next, end) = (int(from).unwrap_or(0), int(next), int(to));
    if next == Some(start) {
        return Err("a range whose next value is its start".into());
    }
    let step = match (next, end) {
        (Some(next), Some(end)) if (next < start) != (end < start) => {
            return Err("a range stepping away from its end".into());
        }
        (Some(next), _) => next.wrapping_sub(start),
        (None, Some(end)) if end < start => -1,
        _ => 1,
    };
    let end = match end {
        Some(end) if exclusive => Bounded::Excluded(end),
        Some(end) => Bounded::Included(end),
        None => Bounded::Unbounded,
    };
    Ok(RangeData::Int(TypedRangeData { start, step, end }))
}

/// A unit's suffix, and the smaller unit and factor a fraction converts to.
type UnitGroup = (&'static str, f64, Option<(f64, f64)>);

/// Filesize units, matched on the uppercased token in this order, each with
/// its bytes, and the smaller unit's bytes and the factor to it.
const FILESIZE_UNITS: [UnitGroup; 13] = [
    ("KB", 1e3, Some((1.0, 1000.0))),
    ("MB", 1e6, Some((1e3, 1000.0))),
    ("GB", 1e9, Some((1e6, 1000.0))),
    ("TB", 1e12, Some((1e9, 1000.0))),
    ("PB", 1e15, Some((1e12, 1000.0))),
    ("EB", 1e18, Some((1e15, 1000.0))),
    ("KIB", 1024.0, Some((1.0, 1024.0))),
    ("MIB", 1_048_576.0, Some((1024.0, 1024.0))),
    ("GIB", 1_073_741_824.0, Some((1_048_576.0, 1024.0))),
    ("TIB", 1_099_511_627_776.0, Some((1_073_741_824.0, 1024.0))),
    ("PIB", 1_125_899_906_842_624.0, Some((1_099_511_627_776.0, 1024.0))),
    ("EIB", 1_152_921_504_606_846_976.0, Some((1_125_899_906_842_624.0, 1024.0))),
    ("B", 1.0, None),
];

/// Duration units, matched as written in this order, each with its
/// nanoseconds, and the smaller unit's nanoseconds and the factor to it.
const DURATION_UNITS: [UnitGroup; 10] = [
    ("ns", 1.0, None),
    ("us", 1e3, Some((1.0, 1000.0))),
    ("\u{b5}s", 1e3, Some((1.0, 1000.0))),
    ("\u{3bc}s", 1e3, Some((1.0, 1000.0))),
    ("ms", 1e6, Some((1e3, 1000.0))),
    ("sec", 1e9, Some((1e6, 1000.0))),
    ("min", 6e10, Some((1e9, 60.0))),
    ("hr", 3.6e12, Some((6e10, 60.0))),
    ("day", 8.64e13, Some((6e10, 1440.0))),
    ("wk", 6.048e14, Some((8.64e13, 7.0))),
];

/// A filesize in bytes: a number and a unit, never a `0x` int.
fn filesize(token: &str, at: usize) -> Shape<i64> {
    if token.starts_with("0x") {
        return Err(Missed::Miss);
    }
    unit_value(&token.to_ascii_uppercase(), at, &FILESIZE_UNITS)
}

/// A duration in nanoseconds: a number and a unit.
fn duration(token: &str, at: usize) -> Shape<i64> {
    unit_value(token, at, &DURATION_UNITS)
}

/// A number with a unit, as nushell's `parse_unit_value` reads one: a whole
/// unit's fraction carried into the smaller unit, the result in the base
/// unit, a fraction of the base unit dropped.
fn unit_value(token: &str, at: usize, units: &[UnitGroup]) -> Shape<i64> {
    let bytes = token.as_bytes();
    let starts_numeric = bytes.len() >= 2
        && (bytes[0].is_ascii_digit()
            || (bytes[0] == b'.' && bytes[1].is_ascii_digit())
            || (bytes[0] == b'-' && bytes[1].is_ascii_digit()));
    if !starts_numeric {
        return Err(Missed::Miss);
    }
    let Some(&(suffix, base, smaller)) = units.iter().find(|(suffix, _, _)| token.ends_with(suffix)) else {
        return Err(Missed::Miss);
    };
    let number = token[..token.len() - suffix.len()].replace('_', "");
    if number.ends_with('$') {
        return Err(Missed::Miss);
    }
    let Ok(number) = number.parse::<f64>() else {
        return fail(at, "a unit's value is a number");
    };
    let (fraction, whole) = modf(number);
    let (factor, count) = match smaller {
        Some((smaller, by)) => (smaller, whole * by + fraction * by),
        None => (base, whole),
    };
    let total = count * factor;
    if (i64::MIN as f64..=i64::MAX as f64).contains(&total) {
        Ok(total as i64)
    } else {
        fail(at, "a value too large for its unit")
    }
}

/// A float's fraction and whole parts, as libm's `modf` splits them.
fn modf(x: f64) -> (f64, f64) {
    let mut u = x.to_bits();
    let e = ((u >> 52) & 0x7ff) as i32 - 0x3ff;
    if e >= 52 {
        if e == 0x400 && (u << 12) != 0 {
            return (x, x);
        }
        u &= 1 << 63;
        return (f64::from_bits(u), x);
    }
    if e < 0 {
        u &= 1 << 63;
        return (x, f64::from_bits(u));
    }
    let mask = (!0u64 >> 12) >> e;
    if u & mask == 0 {
        u &= 1 << 63;
        return (f64::from_bits(u), x);
    }
    u &= !mask;
    let whole = f64::from_bits(u);
    (x - whole, whole)
}

/// A date: RFC 3339, a date alone at midnight UTC, or a date and time
/// without an offset in UTC.
fn datetime(token: &str) -> Shape<DateData> {
    let bytes = token.as_bytes();
    if bytes.len() < 6 || !bytes[..4].iter().all(u8::is_ascii_digit) || bytes[4] != b'-' {
        return Err(Missed::Miss);
    }
    rfc3339(token)
        .or_else(|| rfc3339(&format!("{token}T00:00:00+00:00")))
        .or_else(|| rfc3339(&format!("{token}+00:00")))
        .ok_or(Missed::Miss)
}

/// A date and time as chrono's RFC 3339 parser reads it.
fn rfc3339(s: &str) -> Option<DateData> {
    let bytes = s.as_bytes();
    if bytes.len() < 19 {
        return None;
    }
    let digits = |from: usize, count: usize| -> Option<u32> {
        bytes[from..from + count]
            .iter()
            .try_fold(0u32, |n, b| b.is_ascii_digit().then(|| n * 10 + u32::from(b - b'0')))
    };
    let year = i64::from(digits(0, 4)?);
    let month = digits(5, 2)?;
    let day = digits(8, 2)?;
    if bytes[4] != b'-' || bytes[7] != b'-' || !matches!(bytes[10], b't' | b'T' | b' ') {
        return None;
    }
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    let hour = digits(11, 2)?;
    let minute = digits(14, 2)?;
    let second = digits(17, 2)?;
    if bytes[13] != b':' || bytes[16] != b':' || hour > 23 || minute > 59 {
        return None;
    }
    let (second, mut nano) = match second {
        60 => (59, 1_000_000_000),
        0..=59 => (second, 0),
        _ => return None,
    };
    let mut rest = &s[19..];
    if let Some(fraction) = rest.strip_prefix('.') {
        let count = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if count == 0 {
            return None;
        }
        let significant: String = fraction[..count.min(9)].chars().chain(std::iter::repeat_n('0', 9 - count.min(9))).collect();
        nano += significant.parse::<u32>().ok()?;
        rest = &fraction[count..];
    }
    let offset = if let Some(after) = rest.strip_prefix(['Z', 'z']) {
        rest = after;
        0
    } else {
        let (negative, after) = match rest.strip_prefix('+') {
            Some(after) => (false, after),
            None => (true, rest.strip_prefix('-').or_else(|| rest.strip_prefix('\u{2212}'))?),
        };
        let b = after.as_bytes();
        if b.len() < 5 || b[2] != b':' {
            return None;
        }
        let hours: i32 = after[..2].parse().ok().filter(|_| b[..2].iter().all(u8::is_ascii_digit))?;
        let minutes: i32 = after[3..5].parse().ok().filter(|_| b[3..5].iter().all(u8::is_ascii_digit))?;
        if minutes >= 60 {
            return None;
        }
        rest = &after[5..];
        let offset = hours * 3600 + minutes * 60;
        if offset >= 86_400 {
            return None;
        }
        if negative { -offset } else { offset }
    };
    if !rest.is_empty() {
        return None;
    }
    let local = days_from_civil(year, month, day) * 86_400
        + i64::from(hour) * 3600
        + i64::from(minute) * 60
        + i64::from(second);
    Some(DateData { seconds: local - i64::from(offset), seconds_fraction: nano, utc_offset_minus: offset })
}

/// The days in a proleptic Gregorian month.
fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The days from 1970-01-01 to a proleptic Gregorian date.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let mp = i64::from((month + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// A cell path: `$.` and its members, each a name or a row, `?` optional
/// and `!` case-insensitive.
fn cell_path(token: &str, at: usize) -> NuonResult<NuVal> {
    let Some(rest) = token.strip_prefix("$.") else {
        return Err(read_error(at, "variables aren't NUON"));
    };
    let mut members = Vec::new();
    if rest.is_empty() {
        return Ok(NuVal::CellPath(members));
    }
    let mut segment = String::new();
    let mut quote: Option<char> = None;
    let mut parts: Vec<(String, bool)> = Vec::new();
    let mut quoted = false;
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some('"') if c == '\\' => {
                segment.push(c);
                if let Some(next) = chars.next() {
                    segment.push(next);
                }
            }
            Some(q) if c == q => {
                segment.push(c);
                quote = None;
            }
            Some(_) => segment.push(c),
            None if matches!(c, '"' | '\'' | '`') => {
                quoted = quoted || segment.is_empty();
                segment.push(c);
                quote = Some(c);
            }
            None if c == '.' => {
                parts.push((std::mem::take(&mut segment), quoted));
                quoted = false;
            }
            None => segment.push(c),
        }
    }
    if quote.is_some() {
        return Err(read_error(at, "an unclosed quote in a cell path"));
    }
    parts.push((segment, quoted));
    for (part, quoted) in parts {
        let mut name = part.as_str();
        let (mut optional, mut insensitive) = (false, false);
        loop {
            if let Some(n) = name.strip_suffix('?').filter(|_| !optional) {
                optional = true;
                name = n;
            } else if let Some(n) = name.strip_suffix('!').filter(|_| !insensitive) {
                insensitive = true;
                name = n;
            } else {
                break;
            }
        }
        if name.is_empty() {
            return Err(read_error(at, "a cell path's member is empty"));
        }
        let member = if quoted {
            let value = string(name, at).map_err(|missed| match missed {
                Missed::Fail(e) => e,
                Missed::Miss => read_error(at, "a cell path's member is a name or a row"),
            })?;
            CellPathMemberData::String { value, optional, case_sensitive: !insensitive }
        } else if let Ok(value) = name.parse::<u64>() {
            CellPathMemberData::Int { value, optional }
        } else {
            CellPathMemberData::String { value: name.to_string(), optional, case_sensitive: !insensitive }
        };
        members.push(member);
    }
    Ok(NuVal::CellPath(members))
}
