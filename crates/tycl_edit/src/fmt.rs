use crate::ast::{Document, Value, Item, Key};
use crate::trivia::Trivia;

pub fn format_document(doc: &Document) -> String {
    let mut out = String::new();
    for (_, item) in &doc.items {
        format_item(&mut out, item);
        out.push('\n');
    }
    for trivia in &doc.trailing_trivia {
        format_trivia(&mut out, trivia);
    }
    out
}

fn format_item(out: &mut String, item: &Item) {
    for trivia in &item.leading_trivia {
        format_trivia(out, trivia);
    }
    format_key(out, &item.key);
    if let Some(name) = &item.target_name {
        out.push('(');
        out.push_str(name);
        out.push(')');
    }
    if let Some(ty) = &item.type_annotation {
        out.push_str(": ");
        out.push_str(ty);
    }
    out.push_str(" = ");
    format_value(out, &item.value, 0);
    if let Some(c) = &item.trailing_comment {
        out.push_str(" //");
        out.push_str(c);
    }
}

fn format_key(out: &mut String, key: &Key) {
    match key {
        Key::Ident(s) => out.push_str(s),
        Key::Builtin(s) => {
            out.push('$');
            out.push_str(s);
        }
        Key::String(s) => {
            // Use basic string with escaping for safety
            out.push('"');
            out.push_str(&escape_string(s));
            out.push('"');
        }
    }
}

fn format_value(out: &mut String, value: &Value, indent: usize) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Integer(n) => out.push_str(&n.to_string()),
        Value::Float(n) => {
            let s = format!("{}", n);
            out.push_str(&s);
        }
        Value::String(s) => {
            // Choose quote style based on content
            if s.contains('\n') || s.contains('"') {
                if s.contains("\"\"\"") {
                    out.push_str("r#\"");
                    out.push_str(s);
                    out.push_str("\"#");
                } else {
                    out.push_str("\"\"\"");
                    out.push_str(s);
                    out.push_str("\"\"\"");
                }
            } else {
                out.push('"');
                out.push_str(&escape_string(s));
                out.push('"');
            }
        }
        Value::Time(tv) => match tv {
            crate::ast::TimeValue::LocalDate(s)
            | crate::ast::TimeValue::LocalTime(s)
            | crate::ast::TimeValue::LocalDateTime(s)
            | crate::ast::TimeValue::OffsetDateTime(s) => out.push_str(s),
        },
        Value::List(items) => {
            out.push('[');
            if !items.is_empty() {
                out.push('\n');
                for item in items {
                    for trivia in &item.leading_trivia {
                        for _ in 0..indent + 1 {
                            out.push_str("  ");
                        }
                        format_trivia_inline(out, trivia);
                    }
                    for _ in 0..indent + 1 {
                        out.push_str("  ");
                    }
                    format_value(out, &item.value, indent + 1);
                    if let Some(c) = &item.trailing_comment {
                        out.push_str(" //");
                        out.push_str(c);
                    }
                    out.push_str(",\n");
                }
                for _ in 0..indent {
                    out.push_str("  ");
                }
            }
            out.push(']');
        }
        Value::Map(entries) => {
            out.push('{');
            if !entries.is_empty() {
                out.push('\n');
                for (key, value) in entries {
                    for trivia in &value.leading_trivia {
                        for _ in 0..indent + 1 {
                            out.push_str("  ");
                        }
                        format_trivia_inline(out, trivia);
                    }
                    for _ in 0..indent + 1 {
                        out.push_str("  ");
                    }
                    format_key(out, &Key::Ident(key.clone()));
                    out.push_str(" = ");
                    format_value(out, &value.value, indent + 1);
                    if let Some(c) = &value.trailing_comment {
                        out.push_str(" //");
                        out.push_str(c);
                    }
                    out.push_str(",\n");
                }
                for _ in 0..indent {
                    out.push_str("  ");
                }
            }
            out.push('}');
        }
        Value::Tuple(items) => {
            out.push('(');
            if !items.is_empty() {
                out.push('\n');
                for item in items {
                    for trivia in &item.leading_trivia {
                        for _ in 0..indent + 1 {
                            out.push_str("  ");
                        }
                        format_trivia_inline(out, trivia);
                    }
                    for _ in 0..indent + 1 {
                        out.push_str("  ");
                    }
                    format_value(out, &item.value, indent + 1);
                    if let Some(c) = &item.trailing_comment {
                        out.push_str(" //");
                        out.push_str(c);
                    }
                    out.push_str(",\n");
                }
                for _ in 0..indent {
                    out.push_str("  ");
                }
            }
            out.push(')');
        }
    }
}

fn format_trivia(out: &mut String, trivia: &Trivia) {
    match trivia {
        Trivia::EmptyLine => out.push('\n'),
        Trivia::CommentLine(text) => {
            out.push_str("//");
            out.push_str(text);
            out.push('\n');
        }
    }
}

fn format_trivia_inline(out: &mut String, trivia: &Trivia) {
    match trivia {
        Trivia::EmptyLine => out.push('\n'),
        Trivia::CommentLine(text) => {
            out.push_str("//");
            out.push_str(text);
            out.push('\n');
        }
    }
}

fn escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

impl std::fmt::Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_document(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Document, Value, Item, Key};
    use crate::trivia::Trivia;
    use indexmap::IndexMap;

    #[test]
    fn format_simple() {
        let mut items = IndexMap::new();
        items.insert(
            "key".to_string(),
            Item {
                key: Key::Ident("key".to_string()),
                target_name: None,
                type_annotation: None,
                value: Value::Integer(42),
                leading_trivia: vec![],
                trailing_comment: None,
            },
        );
        let doc = Document {
            items,
            trailing_trivia: vec![],
        };
        assert_eq!(format_document(&doc), "key = 42\n");
    }

    #[test]
    fn format_with_comments() {
        let mut items = IndexMap::new();
        items.insert(
            "key".to_string(),
            Item {
                key: Key::Ident("key".to_string()),
                target_name: None,
                type_annotation: None,
                value: Value::Integer(42),
                leading_trivia: vec![
                    Trivia::CommentLine(" header".to_string()),
                    Trivia::EmptyLine,
                ],
                trailing_comment: Some(" inline".to_string()),
            },
        );
        let doc = Document {
            items,
            trailing_trivia: vec![Trivia::CommentLine(" footer".to_string())],
        };
        let expected = "// header\n\nkey = 42 // inline\n// footer\n";
        assert_eq!(format_document(&doc), expected);
    }
}
