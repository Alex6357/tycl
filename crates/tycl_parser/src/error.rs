use crate::span::Span;

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum ParseError {
    #[error("unexpected token {found:?}, expected {expected:?} at {span:?}")]
    UnexpectedToken {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("invalid integer literal at {span:?}: {detail}")]
    InvalidInteger { detail: String, span: Span },

    #[error("invalid float literal at {span:?}: {detail}")]
    InvalidFloat { detail: String, span: Span },

    #[error("unterminated string at {span:?}")]
    UnterminatedString { span: Span },

    #[error("invalid escape sequence at {span:?}")]
    InvalidEscape { span: Span },

    #[error("duplicate key {key:?} at {span:?}")]
    DuplicateKey { key: String, span: Span },

    #[error("invalid raw string delimiter at {span:?}")]
    InvalidRawString { span: Span },

    #[error("aligned string missing trigger '|' at {span:?}")]
    MissingAlignmentTrigger { span: Span },

    #[error("type mismatch at {span:?}: expected {expected}, found {found}")]
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("record validation failed at {span:?}: {detail}")]
    RecordValidationFailed { detail: String, span: Span },

    #[error("tuple arity mismatch at {span:?}: expected {expected} elements, found {found}")]
    TupleArityMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },

    #[error("missing record field {field:?} at {span:?}")]
    MissingRecordField { field: String, span: Span },
}

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum ValueAccessError {
    #[error("expected {expected}, found {found}")]
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    #[error("missing field {field}")]
    MissingField { field: String },
    #[error("tuple index {index} out of bounds, length is {len}")]
    TupleIndexOutOfBounds { index: usize, len: usize },
}
