use crate::ast::{AnnotatedValue, Document, Value, Item, Key};
use crate::lexer::{Lexer, Token};
use crate::span::Span;
use crate::trivia::Trivia;
use indexmap::IndexMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedToken {
        expected: String,
        found: String,
    },
    UnexpectedEof {
        expected: String,
    },
    DuplicateKey {
        key: String,
    },
    InvalidString,
}

pub struct Parser<'src> {
    source: &'src str,
    tokens: Vec<(Token, Span)>,
    pos: usize,
}

impl<'src> Parser<'src> {
    pub fn new(source: &'src str) -> Self {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        while let Some((tok, span)) = lexer.next_token() {
            tokens.push((tok, span));
        }
        Self {
            source,
            tokens,
            pos: 0,
        }
    }

    fn peek(&self) -> Option<&(Token, Span)> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) -> (Token, Span) {
        let t = self.tokens[self.pos].clone();
        self.pos += 1;
        t
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn at(&self, kind: &Token) -> bool {
        self.peek().map(|(t, _)| t) == Some(kind)
    }

    fn expect(&mut self, kind: Token) -> Result<Span, ParseError> {
        if self.at(&kind) {
            Ok(self.bump().1)
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", kind),
                found: self.found_name(),
            })
        }
    }

    fn found_name(&self) -> String {
        self.peek()
            .map(|(t, _)| format!("{:?}", t))
            .unwrap_or_else(|| "EOF".to_string())
    }

    fn collect_trivia(&mut self) -> Vec<Trivia> {
        let mut trivia = Vec::new();
        let mut after_comment = false;
        while let Some((tok, _)) = self.peek() {
            match tok {
                Token::Newline => {
                    self.pos += 1;
                    if after_comment {
                        after_comment = false;
                    } else {
                        trivia.push(Trivia::EmptyLine);
                    }
                }
                Token::Comment(text) => {
                    let text = text.clone();
                    self.pos += 1;
                    trivia.push(Trivia::CommentLine(text));
                    after_comment = true;
                }
                _ => break,
            }
        }
        trivia
    }

    fn consume_trailing_comment(&mut self) -> Option<String> {
        if let Some((Token::Comment(text), _)) = self.peek() {
            let text = text.clone();
            self.pos += 1;
            Some(text)
        } else {
            None
        }
    }

    fn consume_line_break(&mut self) {
        if self.at(&Token::Newline) {
            self.pos += 1;
        }
    }

    pub fn parse_document(&mut self) -> Result<Document, ParseError> {
        let mut items = IndexMap::new();
        let mut leading_trivia = self.collect_trivia();

        while !self.at_eof() {
            let item = self.parse_item(leading_trivia)?;
            let key_str = item.key.as_str().to_string();
            if items.insert(key_str.clone(), item).is_some() {
                return Err(ParseError::DuplicateKey { key: key_str });
            }
            // consume the line break that ends this item's line
            self.consume_line_break();
            leading_trivia = self.collect_trivia();
        }

        Ok(Document {
            items,
            trailing_trivia: leading_trivia,
        })
    }

    fn parse_item(&mut self, leading_trivia: Vec<Trivia>) -> Result<Item, ParseError> {
        let (key, target_name, type_annotation) = self.parse_key_header()?;
        self.expect(Token::Eq)?;
        let annotated = self.parse_annotated_value()?;
        Ok(Item {
            key,
            target_name,
            type_annotation,
            value: annotated.value,
            leading_trivia,
            trailing_comment: annotated.trailing_comment,
        })
    }

    fn parse_key_header(
        &mut self,
    ) -> Result<(Key, Option<String>, Option<String>), ParseError> {
        let key = self.parse_key()?;

        let target_name = if self.at(&Token::LParen) {
            let start = self.bump().1.end;
            let mut depth = 1;
            let mut end = start;
            while depth > 0 {
                if self.at_eof() {
                    return Err(ParseError::UnexpectedEof {
                        expected: ")".to_string(),
                    });
                }
                let (_, span) = self.bump();
                end = span.start;
                match self.tokens[self.pos - 1].0 {
                    Token::LParen => depth += 1,
                    Token::RParen => depth -= 1,
                    _ => {}
                }
            }
            let inner = &self.source[start as usize..end as usize];
            Some(inner.trim().to_string())
        } else {
            None
        };

        let type_annotation = if self.at(&Token::Colon) {
            self.bump(); // :
            let start = self.pos;
            let mut depth = 0;
            while !self.at_eof() {
                if let Some((tok, _)) = self.peek() {
                    match tok {
                        Token::LParen | Token::LBracket | Token::LBrace => depth += 1,
                        Token::RParen | Token::RBracket | Token::RBrace => {
                            if depth == 0 {
                                break;
                            }
                            depth -= 1;
                        }
                        Token::Eq | Token::Comma => {
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                self.pos += 1;
            }
            let end = self.pos;
            let _raw_tokens: Vec<_> = self.tokens[start..end]
                .iter()
                .map(|(t, _)| format!("{:?}", t))
                .collect();
            // Better: reconstruct from spans
            if start < end {
                let first_span = self.tokens[start].1;
                let last_span = self.tokens[end - 1].1;
                let text = &self.source[first_span.start as usize..last_span.end as usize];
                Some(text.to_string())
            } else {
                None
            }
        } else {
            None
        };

        Ok((key, target_name, type_annotation))
    }

    fn parse_key(&mut self) -> Result<Key, ParseError> {
        match self.peek() {
            Some((Token::BuiltinKey(name), _)) => {
                let name = name.clone();
                self.bump();
                Ok(Key::Builtin(name))
            }
            Some((Token::Ident(name), _)) => {
                let name = name.clone();
                self.bump();
                Ok(Key::Ident(name))
            }
            Some((Token::String(raw), _)) => {
                let decoded = decode_string(raw)?;
                self.bump();
                Ok(Key::String(decoded))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "key".to_string(),
                found: self.found_name(),
            }),
        }
    }

    fn parse_annotated_value(&mut self) -> Result<AnnotatedValue, ParseError> {
        let leading_trivia = self.collect_trivia();
        let value = self.parse_value()?;
        let trailing_comment = self.consume_trailing_comment();
        Ok(AnnotatedValue {
            leading_trivia,
            value,
            trailing_comment,
        })
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        match self.peek() {
            Some((Token::KwNull, _)) => {
                self.bump();
                Ok(Value::Null)
            }
            Some((Token::KwTrue, _)) => {
                self.bump();
                Ok(Value::Bool(true))
            }
            Some((Token::KwFalse, _)) => {
                self.bump();
                Ok(Value::Bool(false))
            }
            Some((Token::Integer(n), _)) => {
                let n = *n;
                self.bump();
                Ok(Value::Integer(n))
            }
            Some((Token::Float(n), _)) => {
                let n = *n;
                self.bump();
                Ok(Value::Float(n))
            }
            Some((Token::String(raw), _)) => {
                let decoded = decode_string(raw)?;
                self.bump();
                Ok(Value::String(decoded))
            }
            Some((Token::Time(tv), _)) => {
                let tv = tv.clone();
                self.bump();
                Ok(Value::Time(tv))
            }
            Some((Token::LBracket, _)) => self.parse_list(),
            Some((Token::LBrace, _)) => self.parse_map(),
            Some((Token::LParen, _)) => self.parse_tuple(),
            _ => Err(ParseError::UnexpectedToken {
                expected: "value".to_string(),
                found: self.found_name(),
            }),
        }
    }

    fn parse_list(&mut self) -> Result<Value, ParseError> {
        self.expect(Token::LBracket)?;
        self.consume_line_break();
        let mut items = Vec::new();
        while !self.at(&Token::RBracket) {
            let annotated = self.parse_annotated_value()?;
            items.push(annotated);
            if self.at(&Token::Comma) {
                self.bump();
                if self.at(&Token::RBracket) {
                    break;
                }
            } else {
                break;
            }
            self.consume_line_break();
        }
        self.expect(Token::RBracket)?;
        Ok(Value::List(items))
    }

    fn parse_map(&mut self) -> Result<Value, ParseError> {
        self.expect(Token::LBrace)?;
        self.consume_line_break();
        let mut entries = Vec::new();
        while !self.at(&Token::RBrace) {
            let key_leading = self.collect_trivia();
            let (key, _, _) = self.parse_key_header()?;
            self.expect(Token::Eq)?;
            let annotated = self.parse_annotated_value()?;
            entries.push((
                key.as_str().to_string(),
                AnnotatedValue {
                    leading_trivia: key_leading,
                    value: annotated.value,
                    trailing_comment: annotated.trailing_comment,
                },
            ));
            if self.at(&Token::Comma) {
                self.bump();
                if self.at(&Token::RBrace) {
                    break;
                }
            } else {
                break;
            }
            self.consume_line_break();
        }
        self.expect(Token::RBrace)?;
        Ok(Value::Map(entries))
    }

    fn parse_tuple(&mut self) -> Result<Value, ParseError> {
        self.expect(Token::LParen)?;
        self.consume_line_break();
        let mut items = Vec::new();
        while !self.at(&Token::RParen) {
            let annotated = self.parse_annotated_value()?;
            items.push(annotated);
            if self.at(&Token::Comma) {
                self.bump();
                if self.at(&Token::RParen) {
                    break;
                }
            } else {
                break;
            }
            self.consume_line_break();
        }
        self.expect(Token::RParen)?;
        Ok(Value::Tuple(items))
    }
}

pub fn parse(source: &str) -> Result<Document, ParseError> {
    let mut parser = Parser::new(source);
    parser.parse_document()
}

impl Document {
    pub fn parse(source: &str) -> Result<Self, ParseError> {
        parse(source)
    }
}

fn decode_string(raw: &str) -> Result<String, ParseError> {
    if raw.starts_with("r|") {
        // aligned raw
        let newline_pos = raw.find('\n').unwrap_or(raw.len());
        let prefix = &raw[..newline_pos];
        let hash_count = prefix[2..]
            .bytes()
            .take_while(|&b| b == b'#')
            .count();
        let closing = format!("{}\"", "#".repeat(hash_count));
        return extract_aligned_lines(&raw[newline_pos..],
            &closing,
        );
    }
    if raw.starts_with("r#") || raw.starts_with("r\"") {
        // raw
        let hash_count = raw[1..]
            .bytes()
            .take_while(|&b| b == b'#')
            .count();
        let closing = format!("{}\"", "#".repeat(hash_count));
        let content_start = 1 + hash_count + 1;
        let content_end = raw.len() - closing.len();
        if content_end >= content_start {
            return Ok(raw[content_start..content_end].to_string());
        }
        return Err(ParseError::InvalidString);
    }
    if raw.starts_with("|\"") {
        // aligned multi-line
        let newline_pos = raw.find('\n').unwrap_or(raw.len());
        return extract_aligned_lines(&raw[newline_pos..],
            "\"\"\"",
        );
    }
    if raw.starts_with("\"\"\"") {
        // multi-line basic
        let content = &raw[3..raw.len() - 3];
        let content = if content.starts_with("\r\n") {
            &content[2..]
        } else if content.starts_with('\n') {
            &content[1..]
        } else {
            content
        };
        return decode_escapes(content);
    }
    if raw.starts_with('"') {
        // basic
        let content = &raw[1..raw.len() - 1];
        return decode_escapes(content);
    }
    Err(ParseError::InvalidString)
}

fn extract_aligned_lines(body: &str, closing: &str) -> Result<String, ParseError> {
    let mut pos = 0;
    if body.starts_with("\r\n") {
        pos += 2;
    } else if body.starts_with('\n') {
        pos += 1;
    }
    let mut result = String::new();
    let remainder = body;
    loop {
        let line_end = remainder[pos..]
            .find('\n')
            .map(|i| pos + i)
            .unwrap_or(remainder.len());
        let line = &remainder[pos..line_end];
        if let Some(trigger_end) = find_ws_trigger(line) {
            let rest = &line[trigger_end..];
            if rest == closing {
                return Ok(result);
            }
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(rest);
            pos = line_end + if line_end < remainder.len() { 1 } else { 0 };
        } else {
            return Err(ParseError::InvalidString);
        }
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

fn decode_escapes(s: &str) -> Result<String, ParseError> {
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
                    return Err(ParseError::InvalidString);
                }
                let code = u32::from_str_radix(&hex, 16).map_err(|_| ParseError::InvalidString)?;
                let ch = char::from_u32(code).ok_or(ParseError::InvalidString)?;
                result.push(ch);
            }
            Some('U') => {
                let hex: String = chars.by_ref().take(8).collect();
                if hex.len() != 8 {
                    return Err(ParseError::InvalidString);
                }
                let code = u32::from_str_radix(&hex, 16).map_err(|_| ParseError::InvalidString)?;
                let ch = char::from_u32(code).ok_or(ParseError::InvalidString)?;
                result.push(ch);
            }
            _ => return Err(ParseError::InvalidString),
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let source = "key = 42\n";
        let doc = parse(source).unwrap();
        assert_eq!(doc.items.len(), 1);
        let item = doc.items.get("key").unwrap();
        assert_eq!(item.value, Value::Integer(42));
    }

    #[test]
    fn test_parse_with_comments() {
        let source = "// header\n\nkey = 42 // inline\n";
        let doc = parse(source).unwrap();
        let item = doc.items.get("key").unwrap();
        assert_eq!(item.leading_trivia.len(), 2);
        assert!(matches!(
            &item.leading_trivia[0],
            Trivia::CommentLine(s) if s == " header"
        ));
        assert!(matches!(
            &item.leading_trivia[1],
            Trivia::EmptyLine
        ));
        assert_eq!(item.trailing_comment.as_deref(), Some(" inline"));
    }

    #[test]
    fn test_parse_list() {
        let source = "my_list = [1, 2, 3]\n";
        let doc = parse(source).unwrap();
        let item = doc.items.get("my_list").unwrap();
        if let Value::List(items) = &item.value {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].value, Value::Integer(1));
            assert_eq!(items[1].value, Value::Integer(2));
            assert_eq!(items[2].value, Value::Integer(3));
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_parse_map() {
        let source = "my_map = { a = 1, b = 2 }\n";
        let doc = parse(source).unwrap();
        let item = doc.items.get("my_map").unwrap();
        if let Value::Map(entries) = &item.value {
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].0, "a");
            assert_eq!(entries[0].1.value, Value::Integer(1));
            assert_eq!(entries[1].0, "b");
            assert_eq!(entries[1].1.value, Value::Integer(2));
        } else {
            panic!("expected map");
        }
    }

    #[test]
    fn test_parse_tuple() {
        let source = "my_tuple = (1, 2, 3)\n";
        let doc = parse(source).unwrap();
        let item = doc.items.get("my_tuple").unwrap();
        if let Value::Tuple(items) = &item.value {
            assert_eq!(items.len(), 3);
        } else {
            panic!("expected tuple");
        }
    }

    #[test]
    fn test_roundtrip_comments_and_newlines() {
        let source = "// header\n\nkey = 42 // inline\n\n// footer\n";
        let doc = parse(source).unwrap();
        let formatted = crate::fmt::format_document(&doc);
        assert_eq!(formatted, source);
    }

    #[test]
    fn test_roundtrip_list_multiline() {
        let source = "my_list = [\n  1,\n  2,\n]\n";
        let doc = parse(source).unwrap();
        let formatted = crate::fmt::format_document(&doc);
        assert_eq!(formatted, source);
    }

    #[test]
    fn test_type_annotation_preserved() {
        let source = "key: int = 42\n";
        let doc = parse(source).unwrap();
        let item = doc.items.get("key").unwrap();
        assert_eq!(item.type_annotation.as_deref(), Some("int"));
    }

    #[test]
    fn test_target_name_preserved() {
        let source = "key(RustKey) = 42\n";
        let doc = parse(source).unwrap();
        let item = doc.items.get("key").unwrap();
        assert_eq!(item.target_name.as_deref(), Some("RustKey"));
    }

    #[test]
    fn test_edit_value() {
        let source = "key = 42\n";
        let mut doc = parse(source).unwrap();
        doc["key"].set_value(100.into());
        assert_eq!(doc["key"].value, Value::Integer(100));
    }

    #[test]
    fn test_insert_and_remove() {
        let source = "key = 42\n";
        let mut doc = parse(source).unwrap();
        doc.insert("new_key", "hello".into());
        assert!(doc.items.contains_key("new_key"));
        doc.remove("key");
        assert!(!doc.items.contains_key("key"));
    }

}
