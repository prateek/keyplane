//! A focused EDN value model with a parser and a deterministic writer.
//!
//! Per ADR 0042 the codec hides parsing and writing behind a Rust boundary, and
//! deterministic save formatting belongs to the app. Rather than depend on a
//! third-party EDN crate whose formatting we cannot control, Keyplane owns a
//! small EDN subset — exactly the value kinds the Profile schema needs
//! (keywords, strings, ints, floats, bools, nil, vectors, maps). The writer
//! emits canonical, stable output so profile diffs stay readable.

use std::fmt::Write as _;

/// An EDN value (the subset Keyplane profiles use).
#[derive(Clone, Debug, PartialEq)]
pub enum Edn {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    /// A keyword without its leading colon, e.g. `schema/version`.
    Keyword(String),
    Vector(Vec<Edn>),
    /// An ordered map. Order is preserved on read and controlled by the codec
    /// on write, which is what makes saves deterministic.
    Map(Vec<(Edn, Edn)>),
}

/// A parse error with a byte offset for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("EDN parse error at byte {pos}: {message}")]
pub struct EdnError {
    pub pos: usize,
    pub message: String,
}

impl Edn {
    pub fn keyword(name: impl Into<String>) -> Edn {
        Edn::Keyword(name.into())
    }

    pub fn string(value: impl Into<String>) -> Edn {
        Edn::Str(value.into())
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Edn::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_keyword(&self) -> Option<&str> {
        match self {
            Edn::Keyword(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Edn::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Edn::Float(f) => Some(*f),
            Edn::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Edn::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_vec(&self) -> Option<&[Edn]> {
        match self {
            Edn::Vector(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&[(Edn, Edn)]> {
        match self {
            Edn::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Look up a keyword key in a map value.
    pub fn get(&self, keyword: &str) -> Option<&Edn> {
        let map = self.as_map()?;
        map.iter()
            .find(|(k, _)| k.as_keyword() == Some(keyword))
            .map(|(_, v)| v)
    }

    /// Parse a single EDN value from `input`.
    pub fn parse(input: &str) -> Result<Edn, EdnError> {
        let mut parser = Parser::new(input);
        parser.skip_ws();
        let value = parser.parse_value()?;
        parser.skip_ws();
        if parser.pos < parser.bytes.len() {
            return Err(parser.err("trailing data after top-level value"));
        }
        Ok(value)
    }

    /// Serialize deterministically with canonical formatting.
    pub fn to_edn_string(&self) -> String {
        let mut out = String::new();
        write_value(&mut out, self, 0);
        out.push('\n');
        out
    }
}

// ---- Parser ----------------------------------------------------------------

struct Parser<'a> {
    bytes: &'a [u8],
    src: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            bytes: src.as_bytes(),
            src,
            pos: 0,
        }
    }

    fn err(&self, message: impl Into<String>) -> EdnError {
        EdnError {
            pos: self.pos,
            message: message.into(),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            // EDN treats commas as whitespace; `;` starts a line comment.
            if c == b';' {
                while let Some(c) = self.peek() {
                    self.pos += 1;
                    if c == b'\n' {
                        break;
                    }
                }
            } else if c.is_ascii_whitespace() || c == b',' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn parse_value(&mut self) -> Result<Edn, EdnError> {
        self.skip_ws();
        match self.peek() {
            None => Err(self.err("unexpected end of input")),
            Some(b'[') => self.parse_seq(b'[', b']').map(Edn::Vector),
            Some(b'(') => self.parse_seq(b'(', b')').map(Edn::Vector),
            Some(b'{') => self.parse_map(),
            Some(b'"') => self.parse_string(),
            Some(b':') => self.parse_keyword(),
            Some(c) if c == b'-' || c == b'+' || c.is_ascii_digit() => self.parse_number_or_symbol(),
            Some(_) => self.parse_symbol_like(),
        }
    }

    fn parse_seq(&mut self, open: u8, close: u8) -> Result<Vec<Edn>, EdnError> {
        debug_assert_eq!(self.peek(), Some(open));
        self.pos += 1;
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                None => return Err(self.err("unterminated collection")),
                Some(c) if c == close => {
                    self.pos += 1;
                    return Ok(items);
                }
                Some(_) => items.push(self.parse_value()?),
            }
        }
    }

    fn parse_map(&mut self) -> Result<Edn, EdnError> {
        self.pos += 1; // consume '{'
        let mut pairs = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                None => return Err(self.err("unterminated map")),
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Edn::Map(pairs));
                }
                Some(_) => {
                    let key = self.parse_value()?;
                    self.skip_ws();
                    if self.peek() == Some(b'}') {
                        return Err(self.err("map has odd number of forms"));
                    }
                    let value = self.parse_value()?;
                    pairs.push((key, value));
                }
            }
        }
    }

    fn parse_string(&mut self) -> Result<Edn, EdnError> {
        self.pos += 1; // consume '"'
        let mut s = String::new();
        loop {
            match self.peek() {
                None => return Err(self.err("unterminated string")),
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(Edn::Str(s));
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(b'"') => s.push('"'),
                        Some(b'\\') => s.push('\\'),
                        Some(b'n') => s.push('\n'),
                        Some(b't') => s.push('\t'),
                        Some(b'r') => s.push('\r'),
                        Some(other) => s.push(other as char),
                        None => return Err(self.err("unterminated escape")),
                    }
                    self.pos += 1;
                }
                Some(_) => {
                    // Copy one UTF-8 scalar.
                    let rest = &self.src[self.pos..];
                    let ch = rest.chars().next().unwrap();
                    s.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }

    fn parse_keyword(&mut self) -> Result<Edn, EdnError> {
        self.pos += 1; // consume ':'
        let start = self.pos;
        self.consume_token();
        if self.pos == start {
            return Err(self.err("empty keyword"));
        }
        Ok(Edn::Keyword(self.src[start..self.pos].to_string()))
    }

    fn parse_number_or_symbol(&mut self) -> Result<Edn, EdnError> {
        let start = self.pos;
        self.consume_token();
        let token = &self.src[start..self.pos];
        parse_number(token).ok_or_else(|| EdnError {
            pos: start,
            message: format!("invalid number: {token}"),
        })
    }

    fn parse_symbol_like(&mut self) -> Result<Edn, EdnError> {
        let start = self.pos;
        self.consume_token();
        let token = &self.src[start..self.pos];
        match token {
            "nil" => Ok(Edn::Nil),
            "true" => Ok(Edn::Bool(true)),
            "false" => Ok(Edn::Bool(false)),
            "" => Err(self.err("unexpected character")),
            other => Err(EdnError {
                pos: start,
                message: format!("unsupported symbol: {other}"),
            }),
        }
    }

    /// Consume a bare token up to the next delimiter or whitespace.
    fn consume_token(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace()
                || c == b','
                || c == b'['
                || c == b']'
                || c == b'{'
                || c == b'}'
                || c == b'('
                || c == b')'
                || c == b'"'
                || c == b';'
            {
                break;
            }
            self.pos += 1;
        }
    }
}

fn parse_number(token: &str) -> Option<Edn> {
    if let Ok(i) = token.parse::<i64>() {
        return Some(Edn::Int(i));
    }
    if let Ok(f) = token.parse::<f64>() {
        if f.is_finite() {
            return Some(Edn::Float(f));
        }
    }
    None
}

// ---- Writer ----------------------------------------------------------------

fn write_value(out: &mut String, value: &Edn, indent: usize) {
    match value {
        Edn::Nil => out.push_str("nil"),
        Edn::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Edn::Int(i) => {
            let _ = write!(out, "{i}");
        }
        Edn::Float(f) => out.push_str(&format_float(*f)),
        Edn::Str(s) => write_string(out, s),
        Edn::Keyword(k) => {
            out.push(':');
            out.push_str(k);
        }
        Edn::Vector(items) => write_vector(out, items, indent),
        Edn::Map(pairs) => write_map(out, pairs, indent),
    }
}

fn is_scalar(value: &Edn) -> bool {
    !matches!(value, Edn::Vector(_) | Edn::Map(_))
}

fn write_vector(out: &mut String, items: &[Edn], indent: usize) {
    if items.is_empty() {
        out.push_str("[]");
        return;
    }
    // A vector of scalars renders inline for readability; nested collections
    // render one element per line.
    if items.iter().all(is_scalar) {
        out.push('[');
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            write_value(out, item, indent);
        }
        out.push(']');
        return;
    }
    out.push('[');
    let child = indent + 1;
    for item in items {
        out.push('\n');
        push_indent(out, child);
        write_value(out, item, child);
    }
    out.push('\n');
    push_indent(out, indent);
    out.push(']');
}

fn write_map(out: &mut String, pairs: &[(Edn, Edn)], indent: usize) {
    if pairs.is_empty() {
        out.push_str("{}");
        return;
    }
    out.push('{');
    let child = indent + 1;
    for (key, value) in pairs {
        out.push('\n');
        push_indent(out, child);
        write_value(out, key, child);
        out.push(' ');
        write_value(out, value, child);
    }
    out.push('\n');
    push_indent(out, indent);
    out.push('}');
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out.push('"');
}

/// Format an f64 as an EDN float that always round-trips back to a float
/// (never an int), so geometry like `1.0` reloads as `Float`, not `Int`.
fn format_float(f: f64) -> String {
    let s = format!("{f}");
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s
    } else {
        format!("{s}.0")
    }
}
