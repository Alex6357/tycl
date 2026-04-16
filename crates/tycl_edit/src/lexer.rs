use crate::span::Span;
use crate::ast::TimeValue;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    // Keywords
    KwStr,
    KwInt,
    KwFloat,
    KwBool,
    KwTime,
    KwMap,
    KwList,
    KwRecord,
    KwTuple,
    KwEnum,
    KwEnv,
    KwAny,
    KwNull,
    KwTrue,
    KwFalse,
    KwLocalDate,
    KwLocalTime,
    KwDateTime,
    KwOffset,

    // Literals
    Ident(String),
    BuiltinKey(String),
    Integer(i64),
    Float(f64),
    Time(TimeValue),
    String(String), // raw source text including quotes

    // Punctuation
    Dollar,
    Colon,
    Eq,
    Comma,
    Question,
    DotDotDot,
    DotDot,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Pipe,

    // Trivia
    Comment(String),
    Newline,
}

pub struct Lexer<'src> {
    input: &'src str,
    pos: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(input: &'src str) -> Self {
        Self { input, pos: 0 }
    }

    pub fn next_token(&mut self) -> Option<(Token, Span)> {
        self.skip_spaces_and_tabs();
        if self.pos >= self.input.len() {
            return None;
        }
        let start = self.pos;

        // Newline
        if self.input[self.pos..].starts_with("\r\n") {
            self.pos += 2;
            return Some((Token::Newline, Span::new(start as u32, self.pos as u32)));
        }
        if self.peek() == Some('\n') {
            self.pos += 1;
            return Some((Token::Newline, Span::new(start as u32, self.pos as u32)));
        }

        // Comment
        if self.input[self.pos..].starts_with("//") {
            self.pos += 2;
            while let Some(c) = self.peek() {
                if c == '\n' {
                    break;
                }
                self.pos += c.len_utf8();
            }
            let text = &self.input[start + 2..self.pos];
            return Some((Token::Comment(text.to_string()), Span::new(start as u32, self.pos as u32)));
        }

        // Keywords and identifiers
        let c = self.peek()?;
        if c.is_ascii_alphabetic() || c == '_' {
            self.pos += c.len_utf8();
            while let Some(ch) = self.peek() {
                if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                    self.pos += ch.len_utf8();
                } else {
                    break;
                }
            }
            let text = &self.input[start..self.pos];
            let tok = match text {
                "str" => Token::KwStr,
                "int" => Token::KwInt,
                "float" => Token::KwFloat,
                "bool" => Token::KwBool,
                "time" => Token::KwTime,
                "map" => Token::KwMap,
                "list" => Token::KwList,
                "record" => Token::KwRecord,
                "tuple" => Token::KwTuple,
                "enum" => Token::KwEnum,
                "env" => Token::KwEnv,
                "any" => Token::KwAny,
                "null" => Token::KwNull,
                "true" => Token::KwTrue,
                "false" => Token::KwFalse,
                "localdate" => Token::KwLocalDate,
                "localtime" => Token::KwLocalTime,
                "datetime" => Token::KwDateTime,
                "offset" => Token::KwOffset,
                _ => Token::Ident(text.to_string()),
            };
            return Some((tok, Span::new(start as u32, self.pos as u32)));
        }

        // Builtin key
        if c == '$' {
            self.pos += 1;
            let mut has_ident = false;
            while let Some(ch) = self.peek() {
                if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                    self.pos += ch.len_utf8();
                    has_ident = true;
                } else {
                    break;
                }
            }
            if has_ident {
                let name = &self.input[start + 1..self.pos];
                return Some((Token::BuiltinKey(name.to_string()), Span::new(start as u32, self.pos as u32)));
            } else {
                self.pos = start + 1;
                return Some((Token::Dollar, Span::new(start as u32, self.pos as u32)));
            }
        }

        // Time literals (try before numbers)
        if let Some(tv) = self.try_time(start) {
            return Some(tv);
        }

        // Numbers
        if c.is_ascii_digit() || c == '+' || c == '-' || c == '.' {
            if let Some(tok) = self.try_number(start) {
                return Some(tok);
            }
        }

        // Strings
        if let Some(tok) = self.try_string(start) {
            return Some(tok);
        }

        // Punctuation
        let tok = match c {
            ':' => { self.pos += 1; Token::Colon }
            '=' => { self.pos += 1; Token::Eq }
            ',' => { self.pos += 1; Token::Comma }
            '?' => { self.pos += 1; Token::Question }
            '(' => { self.pos += 1; Token::LParen }
            ')' => { self.pos += 1; Token::RParen }
            '[' => { self.pos += 1; Token::LBracket }
            ']' => { self.pos += 1; Token::RBracket }
            '{' => { self.pos += 1; Token::LBrace }
            '}' => { self.pos += 1; Token::RBrace }
            '|' => { self.pos += 1; Token::Pipe }
            _ => {
                // Try multi-char punctuation
                if self.input[self.pos..].starts_with("...") {
                    self.pos += 3;
                    Token::DotDotDot
                } else if self.input[self.pos..].starts_with("..") {
                    self.pos += 2;
                    Token::DotDot
                } else {
                    self.pos += c.len_utf8();
                    return self.next_token(); // skip unknown char
                }
            }
        };
        Some((tok, Span::new(start as u32, self.pos as u32)))
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn skip_spaces_and_tabs(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn try_number(&mut self, start: usize) -> Option<(Token, Span)> {
        let c = self.peek()?;

        // Dot-start float: .123
        if c == '.' {
            let next = self.input[self.pos + 1..].chars().next()?;
            if next.is_ascii_digit() {
                self.pos += 1;
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_digit() || ch == '_' {
                        self.pos += ch.len_utf8();
                    } else { break; }
                }
                if let Some(ch) = self.peek() {
                    if ch == 'e' || ch == 'E' {
                        self.pos += 1;
                        if let Some(sign) = self.peek() {
                            if sign == '+' || sign == '-' {
                                self.pos += sign.len_utf8();
                            }
                        }
                        while let Some(ch2) = self.peek() {
                            if ch2.is_ascii_digit() || ch2 == '_' {
                                self.pos += ch2.len_utf8();
                            } else { break; }
                        }
                    }
                }
                let text = &self.input[start..self.pos];
                let f: f64 = text.replace('_', "").parse().ok()?;
                if f.is_finite() { return Some((Token::Float(f), Span::new(start as u32, self.pos as u32))); }
            }
            self.pos = start;
            return None;
        }

        // Optional sign
        if c == '+' || c == '-' {
            self.pos += c.len_utf8();
        }

        let mut has_digits = false;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == '_' {
                self.pos += ch.len_utf8();
                has_digits = true;
            } else { break; }
        }
        if !has_digits {
            self.pos = start;
            return None;
        }

        // Fractional part
        let mut is_float = false;
        if let Some(ch) = self.peek() {
            if ch == '.' {
                let next = self.input[self.pos + 1..].chars().next();
                if next.map(|x| x.is_ascii_digit()).unwrap_or(false) {
                    is_float = true;
                    self.pos += 1; // .
                    while let Some(ch2) = self.peek() {
                        if ch2.is_ascii_digit() || ch2 == '_' {
                            self.pos += ch2.len_utf8();
                        } else { break; }
                    }
                }
            }
        }

        // Exponent part
        if let Some(ch) = self.peek() {
            if ch == 'e' || ch == 'E' {
                is_float = true;
                self.pos += 1;
                if let Some(sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        self.pos += sign.len_utf8();
                    }
                }
                let mut exp_digits = false;
                while let Some(ch2) = self.peek() {
                    if ch2.is_ascii_digit() || ch2 == '_' {
                        self.pos += ch2.len_utf8();
                        exp_digits = true;
                    } else { break; }
                }
                if !exp_digits {
                    self.pos = start;
                    return None;
                }
            }
        }

        let text = &self.input[start..self.pos];
        if is_float {
            let f: f64 = text.replace('_', "").parse().ok()?;
            if f.is_finite() { Some((Token::Float(f), Span::new(start as u32, self.pos as u32))) } else { None }
        } else {
            let n: i64 = text.replace('_', "").parse().ok()?;
            Some((Token::Integer(n), Span::new(start as u32, self.pos as u32)))
        }
    }

    fn try_time(&mut self, start: usize) -> Option<(Token, Span)> {
        // Try offset datetime: YYYY-MM-DDTHH:MM:SS(.frac)?(Z|[+-]HH:MM)
        if let Some(m) = regex_like_offset_datetime(&self.input[self.pos..]) {
            self.pos += m.len();
            return Some((Token::Time(TimeValue::OffsetDateTime(m.to_string())), Span::new(start as u32, self.pos as u32)));
        }
        // Try local datetime: YYYY-MM-DDTHH:MM:SS(.frac)?
        if let Some(m) = regex_like_local_datetime(&self.input[self.pos..]) {
            self.pos += m.len();
            return Some((Token::Time(TimeValue::LocalDateTime(m.to_string())), Span::new(start as u32, self.pos as u32)));
        }
        // Try local date: YYYY-MM-DD
        if let Some(m) = regex_like_local_date(&self.input[self.pos..]) {
            self.pos += m.len();
            return Some((Token::Time(TimeValue::LocalDate(m.to_string())), Span::new(start as u32, self.pos as u32)));
        }
        // Try local time: HH:MM:SS(.frac)?
        if let Some(m) = regex_like_local_time(&self.input[self.pos..]) {
            self.pos += m.len();
            return Some((Token::Time(TimeValue::LocalTime(m.to_string())), Span::new(start as u32, self.pos as u32)));
        }
        None
    }

    fn try_string(&mut self, start: usize) -> Option<(Token, Span)> {
        // Aligned multi-line raw: r|#"
        if self.input[self.pos..].starts_with("r|") {
            let after_r = &self.input[start + 1..];
            if let Some(pipe_idx) = after_r.find('|') {
                let hash_count = after_r[pipe_idx + 1..]
                    .bytes()
                    .take_while(|&b| b == b'#')
                    .count();
                if after_r[pipe_idx + 1 + hash_count..].starts_with('"') {
                    // Valid start
                    self.pos += 1 + pipe_idx + 1 + hash_count + 1; // r|#"
                    let closing = format!("{}\"", "#".repeat(hash_count));
                    let remainder = &self.input[self.pos..];
                    let mut pos = 0;
                    if remainder.starts_with("\r\n") {
                        pos += 2;
                    } else if remainder.starts_with('\n') {
                        pos += 1;
                    }
                    loop {
                        let line_end = remainder[pos..]
                            .find('\n')
                            .map(|i| pos + i)
                            .unwrap_or(remainder.len());
                        let line = &remainder[pos..line_end];
                        if let Some(trigger_end) = find_ws_trigger(line) {
                            let rest = &line[trigger_end..];
                            if rest == closing {
                                pos = line_end + if line_end < remainder.len() { 1 } else { 0 };
                                self.pos += pos;
                                return Some((Token::String(self.input[start..self.pos].to_string()), Span::new(start as u32, self.pos as u32)));
                            }
                            pos = line_end + if line_end < remainder.len() { 1 } else { 0 };
                        } else {
                            return None; // unterminated
                        }
                    }
                }
            }
        }

        // Aligned multi-line: |"""
        if self.input[self.pos..].starts_with("|\"") {
            if self.input[start + 1..].starts_with("\"\"") {
                self.pos += 4; // |"""
                let remainder = &self.input[self.pos..];
                let mut pos = 0;
                if remainder.starts_with("\r\n") {
                    pos += 2;
                } else if remainder.starts_with('\n') {
                    pos += 1;
                }
                loop {
                    let line_end = remainder[pos..]
                        .find('\n')
                        .map(|i| pos + i)
                        .unwrap_or(remainder.len());
                    let line = &remainder[pos..line_end];
                    if let Some(trigger_end) = find_ws_trigger(line) {
                        let rest = &line[trigger_end..];
                        if rest == "\"\"\"" {
                            pos = line_end + if line_end < remainder.len() { 1 } else { 0 };
                            self.pos += pos;
                            return Some((Token::String(self.input[start..self.pos].to_string()), Span::new(start as u32, self.pos as u32)));
                        }
                        pos = line_end + if line_end < remainder.len() { 1 } else { 0 };
                    } else {
                        return None;
                    }
                }
            }
        }

        // Raw string: r#*"
        if self.input[self.pos..].starts_with("r#") || self.input[self.pos..].starts_with("r\"") {
            self.pos += 1; // r
            let hash_count = self.input[self.pos..]
                .bytes()
                .take_while(|&b| b == b'#')
                .count();
            self.pos += hash_count;
            if self.input[self.pos..].starts_with('"') {
                self.pos += 1; // "
                let closing = format!("{}\"", "#".repeat(hash_count));
                if let Some(end) = self.input[self.pos..].find(&closing) {
                    self.pos += end + closing.len();
                    return Some((Token::String(self.input[start..self.pos].to_string()), Span::new(start as u32, self.pos as u32)));
                }
            }
            self.pos = start;
            return None;
        }

        // Multi-line basic: """
        if self.input[self.pos..].starts_with("\"\"\"") {
            self.pos += 3;
            if let Some(end) = self.input[self.pos..].find("\"\"\"") {
                self.pos += end + 3;
                return Some((Token::String(self.input[start..self.pos].to_string()), Span::new(start as u32, self.pos as u32)));
            }
            return None;
        }

        // Basic string: "
        if self.peek() == Some('"') {
            self.pos += 1;
            let mut escaped = false;
            while let Some(c) = self.peek() {
                if escaped {
                    escaped = false;
                    self.pos += c.len_utf8();
                    continue;
                }
                if c == '\\' {
                    escaped = true;
                    self.pos += 1;
                    continue;
                }
                if c == '"' {
                    self.pos += 1;
                    return Some((Token::String(self.input[start..self.pos].to_string()), Span::new(start as u32, self.pos as u32)));
                }
                self.pos += c.len_utf8();
            }
            return None;
        }

        None
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

// Simple prefix-based time matchers (no regex dependency)
fn regex_like_local_date(s: &str) -> Option<&str> {
    let b = s.as_bytes();
    if b.len() < 10 { return None; }
    if !b[0..4].iter().all(|c| c.is_ascii_digit()) { return None; }
    if b[4] != b'-' { return None; }
    if !b[5..7].iter().all(|c| c.is_ascii_digit()) { return None; }
    if b[7] != b'-' { return None; }
    if !b[8..10].iter().all(|c| c.is_ascii_digit()) { return None; }
    Some(&s[..10])
}

fn regex_like_local_time(s: &str) -> Option<&str> {
    let b = s.as_bytes();
    if b.len() < 8 { return None; }
    if !b[0..2].iter().all(|c| c.is_ascii_digit()) { return None; }
    if b[2] != b':' { return None; }
    if !b[3..5].iter().all(|c| c.is_ascii_digit()) { return None; }
    if b[5] != b':' { return None; }
    if !b[6..8].iter().all(|c| c.is_ascii_digit()) { return None; }
    let mut len = 8;
    // optional fraction
    if b.len() > len && b[len] == b'.' {
        len += 1;
        while len < b.len() && b[len].is_ascii_digit() {
            len += 1;
        }
    }
    Some(&s[..len])
}

fn regex_like_local_datetime(s: &str) -> Option<&str> {
    let date = regex_like_local_date(s)?;
    let rest = &s[date.len()..];
    if !rest.starts_with('T') { return None; }
    let time = regex_like_local_time(&rest[1..])?;
    Some(&s[..date.len() + 1 + time.len()])
}

fn regex_like_offset_datetime(s: &str) -> Option<&str> {
    let dt = regex_like_local_datetime(s)?;
    let rest = &s[dt.len()..];
    if rest.starts_with('Z') {
        return Some(&s[..dt.len() + 1]);
    }
    if rest.len() < 6 { return None; }
    let b = rest.as_bytes();
    if b[0] != b'+' && b[0] != b'-' { return None; }
    if !b[1..3].iter().all(|c| c.is_ascii_digit()) { return None; }
    if b[3] != b':' { return None; }
    if !b[4..6].iter().all(|c| c.is_ascii_digit()) { return None; }
    Some(&s[..dt.len() + 6])
}
