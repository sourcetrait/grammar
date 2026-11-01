//! Type text read as nushell reads a type annotation.
use crate::*;

impl NuType {
    /// The type the text names, as nushell's `parse_shape_name` reads it.
    pub fn from_text(text: &str) -> NuTypeResult<Self> {
        let mut reader = Reader { text, pos: 0 };
        reader.skip_blank();
        let ty = reader.ty()?;
        reader.skip_blank();
        if reader.pos < text.len() {
            return Err(read_error(reader.pos, "text after the type"));
        }
        Ok(ty)
    }
}

impl NuTypeFile {
    /// A `.nutype` file: its leading comment's lines as documentation, then
    /// its type.
    pub fn from_text(text: &str) -> NuTypeResult<Self> {
        let mut doc = Vec::new();
        let mut at = 0;
        for line in text.split_inclusive('\n') {
            let Some(comment) = line.trim_end_matches(['\n', '\r']).strip_prefix('#') else {
                break;
            };
            doc.push(comment.strip_prefix(' ').unwrap_or(comment).to_string());
            at += line.len();
        }
        let ty = NuType::from_text(&text[at..]).map_err(|e| match e {
            NuTypeError::Read { offset, reason } => NuTypeError::Read { offset: offset + at, reason },
            other => other,
        })?;
        Ok(Self { doc, ty })
    }
}

/// A read error at `offset`.
fn read_error(offset: usize, reason: impl Into<String>) -> NuTypeError {
    NuTypeError::Read { offset, reason: reason.into() }
}

/// A cursor over type text.
struct Reader<'a> {
    text: &'a str,
    pos: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }

    /// Past whitespace and comments.
    fn skip_blank(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                self.pos += 1;
            } else if c == b'#' {
                while self.peek().is_some_and(|c| c != b'\n') {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    /// Past whitespace, comments, and the commas that separate parameters.
    fn skip_separators(&mut self) {
        loop {
            self.skip_blank();
            if self.peek() != Some(b',') {
                break;
            }
            self.pos += 1;
        }
    }

    /// A type: a name, and its parameters where it is generic.
    fn ty(&mut self) -> NuTypeResult<NuType> {
        let at = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_')) {
            self.pos += 1;
        }
        let name = &self.text[at..self.pos];
        let scalar = match name {
            "any" => NuType::Any,
            "binary" => NuType::Binary,
            "bool" => NuType::Bool,
            "cell-path" => NuType::CellPath,
            "datetime" => NuType::Date,
            "directory" => NuType::Directory,
            "duration" => NuType::Duration,
            "float" => NuType::Float,
            "filesize" => NuType::Filesize,
            "glob" => NuType::Glob,
            "int" => NuType::Int,
            "nothing" => NuType::Nothing,
            "number" => NuType::Number,
            "path" => NuType::Path,
            "range" => NuType::Range,
            "string" => NuType::String,
            "list" | "record" | "table" | "oneof" => return self.generic(name, at),
            "block" | "closure" | "error" | "external_arg" => {
                return Err(NuTypeError::Unsupported { shape: name.to_string() });
            }
            "" => return Err(read_error(at, "a type belongs here")),
            other => return Err(read_error(at, format!("`{other}` names no type"))),
        };
        if self.peek() == Some(b'<') {
            return Err(read_error(self.pos, format!("`{name}` takes no parameters")));
        }
        Ok(scalar)
    }

    /// A generic type: `list`, `record`, `table`, or `oneof`, its parameters
    /// in angle brackets or none.
    fn generic(&mut self, name: &str, at: usize) -> NuTypeResult<NuType> {
        if self.peek() != Some(b'<') {
            return Ok(match name {
                "list" => NuType::List(Box::new(NuType::Any)),
                "record" => NuType::Record(Vec::new()),
                "table" => NuType::Table(Vec::new()),
                _ => NuType::OneOf(Vec::new()),
            });
        }
        self.pos += 1;
        match name {
            "record" => Ok(NuType::Record(self.fields()?)),
            "table" => Ok(NuType::Table(self.fields()?)),
            _ => {
                let mut members = self.members()?;
                if name == "oneof" {
                    return Ok(NuType::OneOf(members));
                }
                if members.len() > 1 {
                    return Err(read_error(at, "a list takes one type"));
                }
                Ok(NuType::List(Box::new(members.pop().unwrap_or(NuType::Any))))
            }
        }
    }

    /// Types up to the closing `>`, separated by commas or whitespace.
    fn members(&mut self) -> NuTypeResult<Vec<NuType>> {
        let mut members = Vec::new();
        loop {
            self.skip_separators();
            match self.peek() {
                Some(b'>') => {
                    self.pos += 1;
                    return Ok(members);
                }
                Some(_) => members.push(self.ty()?),
                None => return Err(read_error(self.pos, "no `>` closes the parameters")),
            }
        }
    }

    /// Fields up to the closing `>`: a name, and `: type` or none for `any`.
    fn fields(&mut self) -> NuTypeResult<Vec<NuField>> {
        let mut fields: Vec<NuField> = Vec::new();
        loop {
            self.skip_separators();
            match self.peek() {
                Some(b'>') => {
                    self.pos += 1;
                    return Ok(fields);
                }
                None => return Err(read_error(self.pos, "no `>` closes the fields")),
                Some(_) => {}
            }
            let name = self.field_name()?;
            self.skip_blank();
            let ty = if self.peek() == Some(b':') {
                self.pos += 1;
                self.skip_blank();
                self.ty()?
            } else {
                NuType::Any
            };
            fields.push(NuField::new(name, ty));
        }
    }

    /// A field's name: double-quoted with escapes, single-quoted, or bare.
    fn field_name(&mut self) -> NuTypeResult<String> {
        let at = self.pos;
        match self.peek() {
            Some(b'"') => {
                let mut name = String::new();
                let mut chars = self.text[at + 1..].char_indices();
                while let Some((i, c)) = chars.next() {
                    match c {
                        '"' => {
                            self.pos = at + 1 + i + 1;
                            return Ok(name);
                        }
                        '\\' => match chars.next() {
                            Some((_, escaped @ ('"' | '\\'))) => name.push(escaped),
                            Some((_, 'n')) => name.push('\n'),
                            Some((_, 't')) => name.push('\t'),
                            _ => return Err(read_error(at + 1 + i, "an escape a field name doesn't take")),
                        },
                        c => name.push(c),
                    }
                }
                Err(read_error(at, "an unclosed quote"))
            }
            Some(quote @ (b'\'' | b'`')) => {
                let close = self.text[at + 1..]
                    .find(char::from(quote))
                    .ok_or_else(|| read_error(at, "an unclosed quote"))?;
                self.pos = at + 1 + close + 1;
                Ok(self.text[at + 1..at + 1 + close].to_string())
            }
            _ => {
                while self.peek().is_some_and(|c| !c.is_ascii_whitespace() && !matches!(c, b':' | b',' | b'>' | b'<' | b'#')) {
                    self.pos += 1;
                }
                let name = &self.text[at..self.pos];
                match name {
                    "" => Err(read_error(at, "a field's name belongs here")),
                    "true" | "false" | "null" => Err(read_error(at, format!("`{name}` isn't a field's name; quote it"))),
                    name => Ok(name.to_string()),
                }
            }
        }
    }
}
