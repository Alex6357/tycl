use crate::error::ParseError;
use crate::lexer::Token;
use crate::parse::Parser;
use crate::span::{Span, Spanned};
use crate::value::Value;

use core::ops::Range;

/// A TyCL type with an optional nullable suffix (`?`).
#[derive(Clone, Debug, PartialEq)]
pub struct Type {
    pub kind: TypeKind,
    pub nullable: bool,
}

/// The concrete kind of a TyCL type.
#[derive(Clone, Debug, PartialEq)]
pub enum TypeKind {
    Str(Option<String>),
    Int(Option<Range<i64>>),
    Float(Option<Range<f64>>),
    Bool,
    Any,
    Time(TimeSubType),
    Map(Box<Spanned<Type>>),
    List(Box<Spanned<Type>>),
    Record(Vec<Spanned<RecordField>>, bool, Option<String>),
    Tuple(Vec<Spanned<Type>>, Option<String>),
    Enum(Vec<Spanned<EnumElement>>, Option<String>),
    Env {
        var_name: Spanned<String>,
        fallback: Option<Box<Spanned<Type>>>,
    },
}

/// Sub-types allowed inside `time(...)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TimeSubType {
    LocalDate,
    LocalTime,
    DateTime,
    Offset,
}

/// A field inside a `record(...)` type.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordField {
    pub name: Spanned<String>,
    pub target_name: Option<Spanned<String>>,
    pub ty: Spanned<Type>,
    pub default: Option<Spanned<Value>>,
}

/// A single element inside an `enum(...)` type.
#[derive(Clone, Debug, PartialEq)]
pub struct EnumElement {
    pub value: Spanned<String>,
    pub target_name: Option<Spanned<String>>,
}

impl<'src> Parser<'src> {
    pub fn parse_type(&mut self) -> Result<Type, ParseError> {
        let kind = self.parse_type_kind()?;
        let nullable = if self.at(&Token::Question) {
            self.bump();
            true
        } else {
            false
        };
        Ok(Type { kind, nullable })
    }

    fn try_parse_struct_name(&mut self) -> Result<Option<String>, ParseError> {
        if !self.at(&Token::LParen) {
            return Ok(None);
        }
        // Save full parser state so we can restore if this isn't a struct-name.
        let saved_lexer = self.lexer.clone();
        let saved_current = self.current.clone();
        let saved_span = self.current_span;
        let saved_last_token_end = self.last_token_end;

        self.bump(); // consume (

        let name = match self.current.clone() {
            Some(Token::Ident(n)) => {
                let n = n.clone();
                self.bump();
                if self.at(&Token::RParen) {
                    self.bump(); // consume )
                    Some(n)
                } else {
                    None
                }
            }
            _ => None,
        };

        if name.is_none() {
            // Restore state
            self.lexer = saved_lexer;
            self.current = saved_current;
            self.current_span = saved_span;
            self.last_token_end = saved_last_token_end;
        }

        Ok(name)
    }

    fn parse_type_kind(&mut self) -> Result<TypeKind, ParseError> {
        match self.current.clone() {
            Some(Token::KwStr) => {
                self.bump();
                let constraint = if self.at(&Token::LParen) {
                    self.bump();
                    let lit = self.parse_spanned_string_lit()?;
                    self.expect(Token::RParen)?;
                    Some(lit.node)
                } else {
                    None
                };
                Ok(TypeKind::Str(constraint))
            }
            Some(Token::KwInt) => {
                self.bump();
                let constraint = if self.at(&Token::LParen) {
                    self.bump();
                    let start = self.expect_int_literal()?;
                    self.expect(Token::DotDot)?;
                    let end = self.expect_int_literal()?;
                    self.expect(Token::RParen)?;
                    Some(start..end)
                } else {
                    None
                };
                Ok(TypeKind::Int(constraint))
            }
            Some(Token::KwFloat) => {
                self.bump();
                let constraint = if self.at(&Token::LParen) {
                    self.bump();
                    let start = self.expect_float_literal()?;
                    self.expect(Token::DotDot)?;
                    let end = self.expect_float_literal()?;
                    self.expect(Token::RParen)?;
                    Some(start..end)
                } else {
                    None
                };
                Ok(TypeKind::Float(constraint))
            }
            Some(Token::KwBool) => {
                self.bump();
                Ok(TypeKind::Bool)
            }
            Some(Token::KwAny) => {
                self.bump();
                Ok(TypeKind::Any)
            }
            Some(Token::KwTime) => {
                self.bump();
                let subtype = if self.at(&Token::LParen) {
                    self.bump();
                    let sub = self.parse_time_subtype()?;
                    self.expect(Token::RParen)?;
                    sub
                } else {
                    TimeSubType::DateTime
                };
                Ok(TypeKind::Time(subtype))
            }
            Some(Token::KwMap) => {
                self.bump();
                self.expect(Token::LParen)?;
                let inner = self.parse_spanned_type()?;
                self.expect(Token::RParen)?;
                Ok(TypeKind::Map(Box::new(inner)))
            }
            Some(Token::KwList) => {
                self.bump();
                self.expect(Token::LParen)?;
                let inner = self.parse_spanned_type()?;
                self.expect(Token::RParen)?;
                Ok(TypeKind::List(Box::new(inner)))
            }
            Some(Token::KwTuple) => {
                self.bump();
                let struct_name = self.try_parse_struct_name()?;
                self.expect(Token::LParen)?;
                let mut elems = Vec::new();
                if !self.at(&Token::RParen) {
                    loop {
                        elems.push(self.parse_spanned_type()?);
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
                Ok(TypeKind::Tuple(elems, struct_name))
            }
            Some(Token::KwRecord) => {
                self.bump();
                let struct_name = self.try_parse_struct_name()?;
                self.expect(Token::LParen)?;
                let mut fields = Vec::new();
                let mut open = false;
                if !self.at(&Token::RParen) {
                    loop {
                        if self.at(&Token::DotDotDot) {
                            self.bump();
                            open = true;
                            break;
                        }
                        fields.push(self.parse_record_field()?);
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
                Ok(TypeKind::Record(fields, open, struct_name))
            }
            Some(Token::KwEnum) => {
                self.bump();
                let struct_name = self.try_parse_struct_name()?;
                self.expect(Token::LParen)?;
                let mut elems = Vec::new();
                if !self.at(&Token::RParen) {
                    loop {
                        elems.push(self.parse_enum_element()?);
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
                Ok(TypeKind::Enum(elems, struct_name))
            }
            Some(Token::KwEnv) => {
                self.bump();
                self.expect(Token::LParen)?;
                let var_name = self.parse_spanned_string_lit()?;
                let fallback = if self.at(&Token::Comma) {
                    self.bump();
                    Some(Box::new(self.parse_spanned_type()?))
                } else {
                    None
                };
                self.expect(Token::RParen)?;
                Ok(TypeKind::Env { var_name, fallback })
            }
            _ => Err(self.unexpected("type")),
        }
    }

    fn parse_spanned_type(&mut self) -> Result<Spanned<Type>, ParseError> {
        let start = self.current_span.start;
        let ty = self.parse_type()?;
        let end = self.current_span.start;
        Ok(Spanned::new(ty, Span::new(start, end)))
    }

    fn parse_time_subtype(&mut self) -> Result<TimeSubType, ParseError> {
        match self.current.clone() {
            Some(Token::KwLocalDate) => {
                self.bump();
                Ok(TimeSubType::LocalDate)
            }
            Some(Token::KwLocalTime) => {
                self.bump();
                Ok(TimeSubType::LocalTime)
            }
            Some(Token::KwDateTime) => {
                self.bump();
                Ok(TimeSubType::DateTime)
            }
            Some(Token::KwOffset) => {
                self.bump();
                Ok(TimeSubType::Offset)
            }
            _ => Err(self.unexpected("time subtype")),
        }
    }

    fn parse_record_field(&mut self) -> Result<Spanned<RecordField>, ParseError> {
        let start = self.current_span.start;
        let (name, name_span) = match self.current.clone() {
            Some(Token::Ident(s)) => {
                let span = self.current_span;
                self.bump();
                (s, span)
            }
            _ => return Err(self.unexpected("identifier")),
        };

        let target_name = self.parse_target_name()?;

        let ty = if self.at(&Token::Colon) {
            self.bump();
            self.parse_spanned_type()?
        } else {
            Spanned::new(
                Type {
                    kind: TypeKind::Any,
                    nullable: false,
                },
                Span::empty(self.current_span.start),
            )
        };

        let default = if self.at(&Token::Eq) {
            self.bump();
            self.push_expected(Some(ty.node.clone()));
            let value = self.parse_value()?;
            self.pop_expected();
            let span = Span::empty(self.current_span.start);
            Some(Spanned::new(value, span))
        } else {
            None
        };
        let end = self.current_span.start;
        Ok(Spanned::new(
            RecordField {
                name: Spanned::new(name, name_span),
                target_name,
                ty,
                default,
            },
            Span::new(start, end),
        ))
    }

    fn parse_enum_element(&mut self) -> Result<Spanned<EnumElement>, ParseError> {
        let start = self.current_span.start;
        let value = self.parse_spanned_string_lit()?;
        let target_name = self.parse_target_name()?;
        let end = self.current_span.start;
        Ok(Spanned::new(
            EnumElement { value, target_name },
            Span::new(start, end),
        ))
    }

    fn parse_spanned_string_lit(&mut self) -> Result<Spanned<String>, ParseError> {
        match self.current.clone() {
            Some(Token::String(lit)) => {
                let span = self.current_span;
                self.bump();
                Ok(Spanned::new(lit, span))
            }
            _ => Err(self.unexpected("string literal")),
        }
    }

    fn expect_int_literal(&mut self) -> Result<i64, ParseError> {
        match self.current.clone() {
            Some(Token::Integer(n)) => {
                self.bump();
                Ok(n)
            }
            Some(Token::Float(f)) => {
                // Lexer may match a trailing-dot number like `0.` as Float
                // because `..` follows it. Accept it if it represents an integer.
                if f.fract() == 0.0 {
                    self.bump();
                    Ok(f as i64)
                } else {
                    Err(self.unexpected("integer literal"))
                }
            }
            _ => Err(self.unexpected("integer literal")),
        }
    }

    fn expect_float_literal(&mut self) -> Result<f64, ParseError> {
        match self.current.clone() {
            Some(Token::Float(n)) => {
                self.bump();
                Ok(n)
            }
            _ => Err(self.unexpected("float literal")),
        }
    }
}

pub fn validate_value_constraints(
    value: &Value,
    expected: &Type,
) -> Result<(), crate::error::ParseError> {
    use crate::error::ParseError;
    use regex::Regex;

    match &expected.kind {
        TypeKind::Int(Some(range)) => {
            if let Value::Integer(n) = value {
                if !range.contains(n) {
                    return Err(ParseError::TypeMismatch {
                        expected: format!("int({}..{})", range.start, range.end),
                        found: n.to_string(),
                        span: crate::span::Span::empty(0),
                    });
                }
            }
        }
        TypeKind::Float(Some(range)) => {
            if let Value::Float(n) = value {
                if !range.contains(n) {
                    return Err(ParseError::TypeMismatch {
                        expected: format!("float({}..{})", range.start, range.end),
                        found: n.to_string(),
                        span: crate::span::Span::empty(0),
                    });
                }
            }
        }
        TypeKind::Str(Some(pattern)) => {
            if let Value::String(s) = value {
                let re = Regex::new(pattern).map_err(|e| ParseError::TypeMismatch {
                    expected: format!("str({})", pattern),
                    found: format!("invalid regex: {}", e),
                    span: crate::span::Span::empty(0),
                })?;
                if !re.is_match(s) {
                    return Err(ParseError::TypeMismatch {
                        expected: format!("str({})", pattern),
                        found: format!("\"{}\"", s),
                        span: crate::span::Span::empty(0),
                    });
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn match_type(inferred: &Type, expected: &Type) -> Result<Type, String> {
    // null is represented as Any with nullable=true
    if inferred.kind == TypeKind::Any && inferred.nullable {
        if expected.nullable {
            return Ok(expected.clone());
        } else {
            return Err("null value for non-nullable type".to_string());
        }
    }

    if expected.nullable {
        // ?T only matches null (handled above) and T
        let non_nullable_expected = Type {
            kind: expected.kind.clone(),
            nullable: false,
        };
        match_type(inferred, &non_nullable_expected)?;
        return Ok(expected.clone());
    }

    match (&inferred.kind, &expected.kind) {
        (_, TypeKind::Any) => Ok(expected.clone()),
        (TypeKind::Bool, TypeKind::Bool) => Ok(expected.clone()),
        (TypeKind::Int(_), TypeKind::Int(_)) => Ok(expected.clone()),
        (TypeKind::Float(_), TypeKind::Float(_)) => Ok(expected.clone()),
        (TypeKind::Str(_), TypeKind::Str(_)) => Ok(expected.clone()),
        (TypeKind::Time(a), TypeKind::Time(b)) => {
            if a == b {
                Ok(expected.clone())
            } else {
                Err(format!("expected time({:?}), found time({:?})", b, a))
            }
        }
        (TypeKind::List(i), TypeKind::List(e)) => {
            match_type(&i.node, &e.node)?;
            Ok(expected.clone())
        }
        (TypeKind::Map(i), TypeKind::Map(e)) => {
            match_type(&i.node, &e.node)?;
            Ok(expected.clone())
        }
        (TypeKind::Tuple(i, _), TypeKind::Tuple(e, _)) => {
            if i.len() != e.len() {
                return Err(format!(
                    "tuple arity mismatch: expected {} elements, found {}",
                    e.len(),
                    i.len()
                ));
            }
            for (a, b) in i.iter().zip(e.iter()) {
                match_type(&a.node, &b.node)?;
            }
            Ok(expected.clone())
        }
        (TypeKind::Record(_, _, _), TypeKind::Record(_, _, _)) => Ok(expected.clone()),
        (TypeKind::Str(_), TypeKind::Enum(_, _)) => Ok(expected.clone()),
        (_, TypeKind::Env { fallback, .. }) => {
            let effective = fallback.as_ref().map(|f| f.node.clone()).unwrap_or(Type {
                kind: TypeKind::Str(None),
                nullable: false,
            });
            match_type(inferred, &effective)?;
            Ok(expected.clone())
        }
        _ => Err(format!(
            "expected {}, found {}",
            type_to_string(expected),
            type_to_string(inferred)
        )),
    }
}

pub fn type_to_string(ty: &Type) -> String {
    let mut s = match &ty.kind {
        TypeKind::Str(_) => "str".to_string(),
        TypeKind::Int(_) => "int".to_string(),
        TypeKind::Float(_) => "float".to_string(),
        TypeKind::Bool => "bool".to_string(),
        TypeKind::Any => "any".to_string(),
        TypeKind::Time(_) => "time".to_string(),
        TypeKind::Map(inner) => format!("map({})", type_to_string(&inner.node)),
        TypeKind::List(inner) => format!("list({})", type_to_string(&inner.node)),
        TypeKind::Record(_, _, _) => "record".to_string(),
        TypeKind::Tuple(elems, _) => {
            let inner: Vec<String> = elems.iter().map(|e| type_to_string(&e.node)).collect();
            format!("tuple({})", inner.join(", "))
        }
        TypeKind::Enum(_, _) => "enum".to_string(),
        TypeKind::Env { .. } => "env".to_string(),
    };
    if ty.nullable {
        s.push('?');
    }
    s
}
