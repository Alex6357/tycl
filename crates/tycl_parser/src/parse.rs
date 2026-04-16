use crate::error::ParseError;
use crate::lexer::Token;
use crate::span::Span;
use crate::span::Spanned;
use crate::types::{TimeSubType, Type, TypeKind, match_type, type_to_string};
use crate::value::TimeValue;
use crate::value::Value;
use logos::Logos;
use std::collections::BTreeMap;

pub struct Parser<'src> {
    pub(crate) lexer: logos::Lexer<'src, Token>,
    pub(crate) current: Option<Token>,
    pub(crate) current_span: Span,
    source: &'src str,
    pub(crate) last_token_end: u32,
    type_stack: Vec<Option<Type>>,
    schema_entries: Option<BTreeMap<String, crate::schema::SchemaEntry>>,
}

impl<'src> Parser<'src> {
    fn next_token(lexer: &mut logos::Lexer<'src, Token>) -> (Option<Token>, Span) {
        match lexer.next() {
            Some(Ok(tok)) => {
                let span = Span::new(lexer.span().start as u32, lexer.span().end as u32);
                (Some(tok), span)
            }
            Some(Err(_)) => {
                let span = Span::new(lexer.span().start as u32, lexer.span().end as u32);
                (Some(Token::Error), span)
            }
            None => {
                let pos = lexer.span().end as u32;
                (None, Span::empty(pos))
            }
        }
    }

    pub fn new(source: &'src str) -> Self {
        let mut lexer = Token::lexer(source);
        let (current, current_span) = Self::next_token(&mut lexer);
        Self {
            lexer,
            current,
            current_span,
            source,
            last_token_end: 0,
            type_stack: Vec::new(),
            schema_entries: None,
        }
    }

    pub(crate) fn bump(&mut self) -> (Option<Token>, Span) {
        self.last_token_end = self.current_span.end;
        let old = (self.current.clone(), self.current_span);
        let (next, span) = Self::next_token(&mut self.lexer);
        self.current = next;
        self.current_span = span;
        old
    }

    pub(crate) fn at(&self, kind: &Token) -> bool {
        self.current.as_ref() == Some(kind)
    }

    fn at_eof(&self) -> bool {
        self.current.is_none()
    }

    pub(crate) fn expect(&mut self, kind: Token) -> Result<Span, ParseError> {
        if self.at(&kind) {
            Ok(self.bump().1)
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", kind),
                found: format!("{:?}", self.current),
                span: self.current_span,
            })
        }
    }

    fn has_newline_since(&self, prev_end: u32) -> bool {
        let slice = &self.source[prev_end as usize..self.current_span.start as usize];
        slice.contains('\n') || slice.contains('\r')
    }

    pub(crate) fn unexpected(&self, expected: &str) -> ParseError {
        ParseError::UnexpectedToken {
            expected: expected.to_string(),
            found: format!("{:?}", self.current),
            span: self.current_span,
        }
    }

    pub fn push_expected(&mut self, ty: Option<Type>) {
        self.type_stack.push(ty);
    }

    pub fn pop_expected(&mut self) -> Option<Option<Type>> {
        self.type_stack.pop()
    }

    pub fn current_expected(&self) -> Option<&Type> {
        self.type_stack.last().and_then(|opt| opt.as_ref())
    }

    pub fn validate_and_coerce(&self, value: Value, inferred: &Type) -> Result<Value, ParseError> {
        if let Some(expected) = self.current_expected() {
            crate::types::validate_value_constraints(&value, expected)?;
            match match_type(inferred, expected) {
                Ok(_) => Ok(value),
                Err(_) => Err(ParseError::TypeMismatch {
                    expected: type_to_string(expected),
                    found: type_to_string(inferred),
                    span: self.current_span,
                }),
            }
        } else {
            Ok(value)
        }
    }

    pub fn parse_document(&mut self) -> Result<crate::value::Document, ParseError> {
        let mut root = BTreeMap::new();

        while !self.at_eof() {
            let pair_end_before = self.last_token_end;
            let (key, value) = self.parse_kv_pair(false)?;
            if root.insert(key.clone(), value).is_some() {
                return Err(ParseError::DuplicateKey {
                    key,
                    span: self.current_span,
                });
            }
            if !self.at_eof() && !self.has_newline_since(pair_end_before) {
                return Err(ParseError::UnexpectedToken {
                    expected: "newline".to_string(),
                    found: format!("{:?}", self.current),
                    span: self.current_span,
                });
            }
        }

        Ok(crate::value::Document { root })
    }

    pub fn parse_schema_document(&mut self) -> Result<crate::schema::Schema, ParseError> {
        self.schema_entries = Some(BTreeMap::new());
        let mut root = BTreeMap::new();

        while !self.at_eof() {
            let pair_end_before = self.last_token_end;
            let (key, value) = self.parse_kv_pair(true)?;
            if root.insert(key.clone(), value).is_some() {
                return Err(ParseError::DuplicateKey {
                    key,
                    span: self.current_span,
                });
            }
            if !self.at_eof() && !self.has_newline_since(pair_end_before) {
                return Err(ParseError::UnexpectedToken {
                    expected: "newline".to_string(),
                    found: format!("{:?}", self.current),
                    span: self.current_span,
                });
            }
        }

        let entries = self.schema_entries.take().unwrap();
        Ok(crate::schema::Schema { entries })
    }

    pub fn parse_document_with_schema(
        &mut self,
        schema: &crate::schema::Schema,
    ) -> Result<crate::value::Document, ParseError> {
        let mut root = BTreeMap::new();
        let mut seen_keys = std::collections::HashSet::new();

        while !self.at_eof() {
            let pair_end_before = self.last_token_end;
            let (key, value) = self.parse_kv_pair_with_schema(schema)?;
            let mut final_key = schema
                .entries
                .get(&key)
                .and_then(|e| e.target_name.clone())
                .unwrap_or_else(|| key.clone());

            // If the schema type is enum and the value is a string, use the enum element's target name.
            if let Some(entry) = schema.entries.get(&key) {
                if let TypeKind::Enum(elems, _) = &entry.ty.kind {
                    if let Value::String(s) = &value {
                        if let Some(elem) = elems.iter().find(|e| e.node.value.node == *s) {
                            if let Some(tn) = &elem.node.target_name {
                                final_key = tn.node.clone();
                            }
                        }
                    }
                }
            }

            if !seen_keys.insert(key.clone()) {
                return Err(ParseError::DuplicateKey {
                    key: final_key,
                    span: self.current_span,
                });
            }

            if root.insert(final_key.clone(), value).is_some() {
                return Err(ParseError::DuplicateKey {
                    key: final_key,
                    span: self.current_span,
                });
            }
            if !self.at_eof() && !self.has_newline_since(pair_end_before) {
                return Err(ParseError::UnexpectedToken {
                    expected: "newline".to_string(),
                    found: format!("{:?}", self.current),
                    span: self.current_span,
                });
            }
        }

        // Fill in defaults for missing keys
        for (key, entry) in &schema.entries {
            if !seen_keys.contains(key) {
                let final_key = entry.target_name.clone().unwrap_or_else(|| key.clone());
                root.insert(final_key, entry.default.clone());
            }
        }

        Ok(crate::value::Document { root })
    }

    pub fn parse_kv_pair(
        &mut self,
        allow_target_name: bool,
    ) -> Result<(String, Value), ParseError> {
        let (key, target_name, annotated_ty) = self.parse_kv_pair_header(allow_target_name)?;
        self.push_expected(annotated_ty.clone());
        self.expect(Token::Eq)?;
        let (value, inferred_ty) = self.parse_value_with_type()?;
        self.pop_expected();

        let effective_ty = annotated_ty.unwrap_or(inferred_ty);

        if let Some(entries) = self.schema_entries.as_mut() {
            entries.insert(
                key.clone(),
                crate::schema::SchemaEntry {
                    target_name: target_name.clone(),
                    ty: effective_ty,
                    default: value.clone(),
                },
            );
        }

        Ok((key, value))
    }

    fn parse_kv_pair_with_schema(
        &mut self,
        schema: &crate::schema::Schema,
    ) -> Result<(String, Value), ParseError> {
        let (key, _target_name, annotated_ty) = self.parse_kv_pair_header(false)?;

        let expected_ty = if let Some(entry) = schema.entries.get(&key) {
            Some(entry.ty.clone())
        } else {
            annotated_ty
        };

        self.push_expected(expected_ty);
        self.expect(Token::Eq)?;
        let (value, _) = self.parse_value_with_type()?;
        self.pop_expected();
        Ok((key, value))
    }

    /// Parse the key, optional target-name, and optional type annotation.
    /// Returns the key string and the optional annotated type.
    pub fn parse_kv_pair_header(
        &mut self,
        allow_target_name: bool,
    ) -> Result<(String, Option<String>, Option<Type>), ParseError> {
        let key = self.parse_key()?;

        let target_name = if self.at(&Token::LParen) {
            if allow_target_name {
                self.parse_target_name()?.map(|s| s.node)
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "type annotation or '='".to_string(),
                    found: format!("{:?}", self.current),
                    span: self.current_span,
                });
            }
        } else {
            None
        };

        // Optional type annotation
        let annotated_ty = if self.at(&Token::Colon) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };

        Ok((key, target_name, annotated_ty))
    }

    fn parse_key(&mut self) -> Result<String, ParseError> {
        match self.current.clone() {
            Some(Token::BuiltinKey(name)) => {
                self.bump();
                Ok(format!("${}", name))
            }
            Some(Token::Ident(name)) => {
                self.bump();
                Ok(name)
            }
            Some(Token::String(s)) => {
                self.bump();
                Ok(s)
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "key".to_string(),
                found: format!("{:?}", self.current),
                span: self.current_span,
            }),
        }
    }

    pub(crate) fn parse_target_name(&mut self) -> Result<Option<Spanned<String>>, ParseError> {
        if self.at(&Token::LParen) {
            let start = self.current_span.start;
            let mut depth = 1;
            self.bump(); // (
            while depth > 0 {
                match self.current {
                    Some(Token::LParen) => depth += 1,
                    Some(Token::RParen) => depth -= 1,
                    None => break,
                    _ => {}
                }
                if depth > 0 {
                    self.bump();
                }
            }
            let end = self.current_span.start; // position of )
            self.expect(Token::RParen)?;
            let inner = &self.source[(start + 1) as usize..end as usize];
            let name = inner.trim().to_string();
            let span = Span::new(start, end + 1);
            Ok(Some(Spanned::new(name, span)))
        } else {
            Ok(None)
        }
    }

    pub fn parse_value(&mut self) -> Result<Value, ParseError> {
        let (value, _) = self.parse_value_with_type()?;
        Ok(value)
    }

    fn parse_value_with_type(&mut self) -> Result<(Value, Type), ParseError> {
        match self.current.clone() {
            Some(Token::Error) => Err(ParseError::UnexpectedToken {
                expected: "value".to_string(),
                found: "lexer error".to_string(),
                span: self.current_span,
            }),
            Some(Token::KwNull) => {
                self.bump();
                let inferred = Type {
                    kind: TypeKind::Any,
                    nullable: true,
                };
                let value = self.validate_and_coerce(Value::Null, &inferred)?;
                Ok((value, inferred))
            }
            Some(Token::KwTrue) => {
                self.bump();
                let inferred = Type {
                    kind: TypeKind::Bool,
                    nullable: false,
                };
                let value = self.validate_and_coerce(Value::Bool(true), &inferred)?;
                Ok((value, inferred))
            }
            Some(Token::KwFalse) => {
                self.bump();
                let inferred = Type {
                    kind: TypeKind::Bool,
                    nullable: false,
                };
                let value = self.validate_and_coerce(Value::Bool(false), &inferred)?;
                Ok((value, inferred))
            }
            Some(Token::Integer(n)) => {
                self.bump();
                let inferred = Type {
                    kind: TypeKind::Int(None),
                    nullable: false,
                };
                let value = self.validate_and_coerce(Value::Integer(n), &inferred)?;
                Ok((value, inferred))
            }
            Some(Token::Float(n)) => {
                self.bump();
                let inferred = Type {
                    kind: TypeKind::Float(None),
                    nullable: false,
                };
                let value = self.validate_and_coerce(Value::Float(n), &inferred)?;
                Ok((value, inferred))
            }
            Some(Token::String(s)) => {
                self.bump();
                // If the expected type is Enum, validate the string is a valid enum element.
                if let Some(expected) = self.current_expected() {
                    if let TypeKind::Enum(elems, _) = &expected.kind {
                        if !elems.iter().any(|e| e.node.value.node == s) {
                            return Err(ParseError::TypeMismatch {
                                expected: type_to_string(expected),
                                found: format!("str(\"{}\")", s),
                                span: self.current_span,
                            });
                        }
                    }
                }
                let inferred = Type {
                    kind: TypeKind::Str(None),
                    nullable: false,
                };
                let value = self.validate_and_coerce(Value::String(s), &inferred)?;
                Ok((value, inferred))
            }
            Some(Token::Time(tv)) => {
                self.bump();
                let subtype = time_value_to_subtype(&tv);
                let inferred = Type {
                    kind: TypeKind::Time(subtype),
                    nullable: false,
                };
                let value = self.validate_and_coerce(Value::Time(tv), &inferred)?;
                Ok((value, inferred))
            }
            Some(Token::LBracket) => self.parse_list(),
            Some(Token::LBrace) => self.parse_map(),
            Some(Token::LParen) => self.parse_tuple(),
            None => Err(self.unexpected("value")),
            _ => Err(self.unexpected("value")),
        }
    }

    fn parse_list(&mut self) -> Result<(Value, Type), ParseError> {
        self.expect(Token::LBracket)?;
        let mut items = Vec::new();
        let mut item_types = Vec::new();

        let inner_expected = self.current_expected().and_then(|ty| match &ty.kind {
            TypeKind::List(inner) => Some(inner.node.clone()),
            TypeKind::Any => Some(Type {
                kind: TypeKind::Any,
                nullable: false,
            }),
            _ => None,
        });

        if !self.at(&Token::RBracket) {
            loop {
                self.push_expected(inner_expected.clone());
                let (item, item_type) = self.parse_value_with_type()?;
                self.pop_expected();
                items.push(item);
                item_types.push(item_type);
                if self.at(&Token::Comma) {
                    self.bump();
                    if self.at(&Token::RBracket) {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RBracket)?;

        let inner_ty = if item_types.is_empty() {
            Type {
                kind: TypeKind::Any,
                nullable: false,
            }
        } else if item_types.iter().all(|t| t == &item_types[0]) {
            item_types[0].clone()
        } else {
            Type {
                kind: TypeKind::Any,
                nullable: false,
            }
        };

        let inferred = Type {
            kind: TypeKind::List(Box::new(Spanned::new(
                inner_ty,
                Span::empty(self.current_span.start),
            ))),
            nullable: false,
        };

        let value = Value::List(items);
        let value = self.validate_and_coerce(value, &inferred)?;
        Ok((value, inferred))
    }

    fn parse_map(&mut self) -> Result<(Value, Type), ParseError> {
        self.expect(Token::LBrace)?;

        let record_ctx = self.current_expected().and_then(|ty| match &ty.kind {
            TypeKind::Record(fields, open, _) => Some((fields.clone(), *open)),
            _ => None,
        });

        let map_inner_ty = self.current_expected().and_then(|ty| match &ty.kind {
            TypeKind::Map(inner) => Some(inner.node.clone()),
            TypeKind::Any => Some(Type {
                kind: TypeKind::Any,
                nullable: false,
            }),
            _ => None,
        });

        let mut entries = BTreeMap::new();
        let mut value_types = Vec::new();

        if !self.at(&Token::RBrace) {
            loop {
                let (key, _target_name, annotated_ty) = self.parse_kv_pair_header(false)?;

                let expected_ty = if let Some((fields, _)) = &record_ctx {
                    let field_ty = fields
                        .iter()
                        .find(|f| f.node.name.node == key)
                        .map(|f| f.node.ty.node.clone());
                    annotated_ty.or(field_ty)
                } else {
                    annotated_ty.or(map_inner_ty.clone())
                };

                self.push_expected(expected_ty);
                self.expect(Token::Eq)?;
                let (value, value_type) = self.parse_value_with_type()?;
                self.pop_expected();

                value_types.push(value_type);

                if entries.insert(key.clone(), value).is_some() {
                    return Err(ParseError::DuplicateKey {
                        key,
                        span: self.current_span,
                    });
                }

                if self.at(&Token::Comma) {
                    self.bump();
                    if self.at(&Token::RBrace) {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RBrace)?;

        if let Some((fields, open)) = record_ctx {
            // Validate all declared fields are present or have default
            for field in &fields {
                let name = &field.node.name.node;
                if !entries.contains_key(name) && field.node.default.is_none() {
                    return Err(ParseError::MissingRecordField {
                        field: name.clone(),
                        span: self.current_span,
                    });
                }
            }

            // For closed records, check no extra fields
            if !open {
                for key in entries.keys() {
                    if !fields.iter().any(|f| &f.node.name.node == key) {
                        return Err(ParseError::RecordValidationFailed {
                            detail: format!("unexpected field '{}' in record", key),
                            span: self.current_span,
                        });
                    }
                }
            }

            // Build Record value
            let mut record_map = BTreeMap::new();
            for field in &fields {
                let lookup_name = field.node.name.node.clone();
                let output_name = field
                    .node
                    .target_name
                    .as_ref()
                    .map(|t| t.node.clone())
                    .unwrap_or_else(|| lookup_name.clone());
                let val = if let Some(v) = entries.remove(&lookup_name) {
                    v
                } else {
                    field.node.default.as_ref().unwrap().node.clone()
                };
                record_map.insert(output_name, val);
            }

            let value = Value::Record(record_map);
            // Validate against the expected record type
            let expected = self.current_expected().unwrap().clone();
            let value = self.validate_and_coerce(value, &expected)?;
            Ok((value, expected))
        } else {
            let inner_ty = if value_types.is_empty() {
                Type {
                    kind: TypeKind::Any,
                    nullable: false,
                }
            } else if value_types.iter().all(|t| t == &value_types[0]) {
                value_types[0].clone()
            } else {
                Type {
                    kind: TypeKind::Any,
                    nullable: false,
                }
            };

            let inferred = Type {
                kind: TypeKind::Map(Box::new(Spanned::new(
                    inner_ty,
                    Span::empty(self.current_span.start),
                ))),
                nullable: false,
            };

            let value = Value::Map(entries);
            let value = self.validate_and_coerce(value, &inferred)?;
            Ok((value, inferred))
        }
    }

    fn parse_tuple(&mut self) -> Result<(Value, Type), ParseError> {
        self.expect(Token::LParen)?;
        let mut items = Vec::new();
        let mut item_types = Vec::new();

        let expected_elems = self.current_expected().and_then(|ty| match &ty.kind {
            TypeKind::Tuple(elems, _) => Some(elems.clone()),
            TypeKind::Any => Some(Vec::new()), // marker: any tuple
            _ => None,
        });

        let mut idx = 0;
        if !self.at(&Token::RParen) {
            loop {
                let elem_expected = expected_elems.as_ref().and_then(|elems| {
                    if elems.is_empty() {
                        Some(Type {
                            kind: TypeKind::Any,
                            nullable: false,
                        })
                    } else {
                        elems.get(idx).map(|spanned| spanned.node.clone())
                    }
                });
                self.push_expected(elem_expected);
                let (item, item_type) = self.parse_value_with_type()?;
                self.pop_expected();
                items.push(item);
                item_types.push(item_type);
                idx += 1;
                if self.at(&Token::Comma) {
                    self.bump();
                    if self.at(&Token::RParen) {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RParen)?;

        // Validate arity for non-any tuple expectations
        if let Some(elems) = expected_elems {
            if !elems.is_empty() && elems.len() != items.len() {
                return Err(ParseError::TupleArityMismatch {
                    expected: elems.len(),
                    found: items.len(),
                    span: self.current_span,
                });
            }
        }

        let inferred = Type {
            kind: TypeKind::Tuple(
                item_types
                    .into_iter()
                    .map(|t| Spanned::new(t, Span::empty(self.current_span.start)))
                    .collect(),
                None,
            ),
            nullable: false,
        };

        let value = Value::Tuple(items);
        let value = self.validate_and_coerce(value, &inferred)?;
        Ok((value, inferred))
    }
}

pub fn parse(source: &str) -> Result<crate::value::Document, ParseError> {
    let mut parser = Parser::new(source);
    parser.parse_document()
}

pub fn parse_schema(source: &str) -> Result<crate::schema::Schema, ParseError> {
    let mut parser = Parser::new(source);
    parser.parse_schema_document()
}

pub fn parse_with_schema(
    source: &str,
    schema: &crate::schema::Schema,
) -> Result<crate::value::Document, ParseError> {
    let mut parser = Parser::new(source);
    parser.parse_document_with_schema(schema)
}

pub fn parse_with_schema_str(
    source: &str,
    schema_source: &str,
) -> Result<crate::value::Document, ParseError> {
    let schema = parse_schema(schema_source)?;
    parse_with_schema(source, &schema)
}

fn time_value_to_subtype(tv: &TimeValue) -> TimeSubType {
    match tv {
        TimeValue::LocalDate(_) => TimeSubType::LocalDate,
        TimeValue::LocalTime(_) => TimeSubType::LocalTime,
        TimeValue::LocalDateTime(_) => TimeSubType::DateTime,
        TimeValue::OffsetDateTime(_) => TimeSubType::Offset,
    }
}
