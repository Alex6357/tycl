use crate::value::TimeValue;
use logos::{Lexer, Logos};

#[derive(Default, Debug, Clone, PartialEq)]
pub enum LexError {
    #[default]
    Other,
    UnterminatedString,
    InvalidEscape,
    InvalidRawString,
    MissingAlignmentTrigger,
    InvalidInteger,
    InvalidFloat,
}

fn parse_int(lex: &mut Lexer<Token>) -> Result<i64, LexError> {
    let s = lex.slice().replace('_', "");
    s.parse().map_err(|_| LexError::InvalidInteger)
}

fn parse_float(lex: &mut Lexer<Token>) -> Result<f64, LexError> {
    let s = lex.slice().replace('_', "");
    let f: f64 = s.parse().map_err(|_| LexError::InvalidFloat)?;
    if f.is_infinite() || f.is_nan() {
        return Err(LexError::InvalidFloat);
    }
    Ok(f)
}

fn decode_escapes(s: &str) -> Result<String, LexError> {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }
        match chars.next() {
            Some('b') => result.push('\x08'),
            Some('f') => result.push('\x0C'),
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('t') => result.push('\t'),
            Some('\\') => result.push('\\'),
            Some('\'') => result.push('\''),
            Some('"') => result.push('"'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                if hex.len() != 4 {
                    return Err(LexError::InvalidEscape);
                }
                let code = u32::from_str_radix(&hex, 16).map_err(|_| LexError::InvalidEscape)?;
                let ch = char::from_u32(code).ok_or(LexError::InvalidEscape)?;
                result.push(ch);
            }
            Some('U') => {
                let hex: String = chars.by_ref().take(8).collect();
                if hex.len() != 8 {
                    return Err(LexError::InvalidEscape);
                }
                let code = u32::from_str_radix(&hex, 16).map_err(|_| LexError::InvalidEscape)?;
                let ch = char::from_u32(code).ok_or(LexError::InvalidEscape)?;
                result.push(ch);
            }
            _ => return Err(LexError::InvalidEscape),
        }
    }
    Ok(result)
}

fn read_basic_string(lex: &mut Lexer<Token>) -> Result<String, LexError> {
    let remainder = lex.remainder();
    let mut escaped = false;
    for (i, c) in remainder.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '"' {
            let content = &remainder[..i];
            lex.bump(i + 1);
            return decode_escapes(content);
        }
    }
    Err(LexError::UnterminatedString)
}

fn read_ml_string(lex: &mut Lexer<Token>) -> Result<String, LexError> {
    let remainder = lex.remainder();
    if let Some(end) = remainder.find("\"\"\"") {
        let mut content = &remainder[..end];
        if content.starts_with("\r\n") {
            content = &content[2..];
        } else if content.starts_with('\n') {
            content = &content[1..];
        }
        lex.bump(end + 3);
        return decode_escapes(content);
    }
    Err(LexError::UnterminatedString)
}

fn read_raw_string(lex: &mut Lexer<Token>) -> Result<String, LexError> {
    let slice = lex.slice();
    let hash_count = slice[1..].bytes().take_while(|&b| b == b'#').count();
    let closing = format!("\"{}", "#".repeat(hash_count));
    let remainder = lex.remainder();
    if let Some(end) = remainder.find(&closing) {
        let content = &remainder[..end];
        lex.bump(end + closing.len());
        Ok(content.to_string())
    } else {
        Err(LexError::UnterminatedString)
    }
}

fn find_ws_trigger(line: &str) -> Option<usize> {
    let mut bytes = 0;
    for c in line.chars() {
        if c == ' ' || c == '\t' {
            bytes += c.len_utf8();
        } else if c == '|' {
            return Some(bytes + c.len_utf8());
        } else {
            return None;
        }
    }
    None
}

fn read_aligned_ml_string(lex: &mut Lexer<Token>) -> Result<String, LexError> {
    let remainder = lex.remainder();
    let mut pos = 0;

    if remainder.starts_with("\r\n") {
        pos += 2;
    } else if remainder.starts_with('\n') {
        pos += 1;
    }

    let mut result = String::new();

    loop {
        let line_end = remainder[pos..]
            .find('\n')
            .map(|i| pos + i)
            .unwrap_or(remainder.len());
        let line = &remainder[pos..line_end];

        if let Some(trigger_end) = find_ws_trigger(line) {
            let rest = &line[trigger_end..];
            if rest == "\"\"\"" {
                lex.bump(line_end + if line_end < remainder.len() { 1 } else { 0 });
                return Ok(result);
            }
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(rest);
            pos = line_end + if line_end < remainder.len() { 1 } else { 0 };
        } else {
            if pos >= remainder.len() {
                return Err(LexError::UnterminatedString);
            }
            return Err(LexError::MissingAlignmentTrigger);
        }
    }
}

fn read_aligned_raw_string(lex: &mut Lexer<Token>) -> Result<String, LexError> {
    let slice = lex.slice();
    let after_r = &slice[1..];
    let pipe_idx = after_r.find('|').unwrap_or(0);
    let hash_count = after_r[pipe_idx + 1..]
        .bytes()
        .take_while(|&b| b == b'#')
        .count();
    let closing = format!("{}\"", "#".repeat(hash_count));
    let remainder = lex.remainder();

    let mut pos = 0;
    if remainder.starts_with("\r\n") {
        pos += 2;
    } else if remainder.starts_with('\n') {
        pos += 1;
    }

    let mut result = String::new();

    loop {
        let line_end = remainder[pos..]
            .find('\n')
            .map(|i| pos + i)
            .unwrap_or(remainder.len());
        let line = &remainder[pos..line_end];

        if let Some(trigger_end) = find_ws_trigger(line) {
            let rest = &line[trigger_end..];
            if rest == closing {
                lex.bump(line_end + if line_end < remainder.len() { 1 } else { 0 });
                return Ok(result);
            }
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(rest);
            pos = line_end + if line_end < remainder.len() { 1 } else { 0 };
        } else {
            if pos >= remainder.len() {
                return Err(LexError::UnterminatedString);
            }
            return Err(LexError::MissingAlignmentTrigger);
        }
    }
}

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(skip r"[ \t\r\n]+|//[^\n]*")]
#[logos(error = LexError)]
pub enum Token {
    #[token("str")]
    KwStr,
    #[token("int")]
    KwInt,
    #[token("float")]
    KwFloat,
    #[token("bool")]
    KwBool,
    #[token("time")]
    KwTime,
    #[token("map")]
    KwMap,
    #[token("list")]
    KwList,
    #[token("record")]
    KwRecord,
    #[token("tuple")]
    KwTuple,
    #[token("enum")]
    KwEnum,
    #[token("env")]
    KwEnv,
    #[token("any")]
    KwAny,
    #[token("null")]
    KwNull,
    #[token("true")]
    KwTrue,
    #[token("false")]
    KwFalse,
    #[token("localdate")]
    KwLocalDate,
    #[token("localtime")]
    KwLocalTime,
    #[token("datetime")]
    KwDateTime,
    #[token("offset")]
    KwOffset,

    #[regex(r"[A-Za-z][A-Za-z0-9_\-]*", |lex| lex.slice().to_string())]
    Ident(String),
    #[regex(r"\$[A-Za-z][A-Za-z0-9_\-]*", |lex| lex.slice()[1..].to_string())]
    BuiltinKey(String),

    #[regex(r"[+-]?[0-9][0-9_]*", parse_int)]
    Integer(i64),
    // NOTE: the first alternative requires at least one digit after the dot
    // so that `0..100` is lexed as `0` + `..` + `100` rather than `0.100`.
    #[regex(r"[+-]?[0-9][0-9_]*\.[0-9][0-9_]*([eE][+-]?[0-9][0-9_]*)?|[+-]?[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*|\.[0-9][0-9_]*([eE][+-]?[0-9][0-9_]*)?", parse_float)]
    Float(f64),

    #[regex(r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?(Z|[+-][0-9]{2}:[0-9]{2})", |lex| TimeValue::OffsetDateTime(lex.slice().to_string()))]
    #[regex(r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?", |lex| TimeValue::LocalDateTime(lex.slice().to_string()))]
    #[regex(r"[0-9]{4}-[0-9]{2}-[0-9]{2}", |lex| TimeValue::LocalDate(lex.slice().to_string()))]
    #[regex(r"[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?", |lex| TimeValue::LocalTime(lex.slice().to_string()))]
    Time(TimeValue),

    #[token("$")]
    Dollar,
    #[token(":")]
    Colon,
    #[token("=")]
    Eq,
    #[token(",")]
    Comma,
    #[token("?")]
    Question,
    #[token("...")]
    DotDotDot,
    #[token("..")]
    DotDot,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("|")]
    Pipe,

    #[token(r##"""##, read_basic_string)]
    #[token("\u{0022}\u{0022}\u{0022}", read_ml_string)]
    #[regex("r#*\u{0022}", read_raw_string)]
    #[token("|\u{0022}\u{0022}\u{0022}", read_aligned_ml_string)]
    #[regex("r\\|#*\u{0022}", read_aligned_raw_string)]
    String(String),
    Error,
}
