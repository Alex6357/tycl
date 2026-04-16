extern crate self as tycl_parser;

mod span;

pub mod error;
pub mod lexer;
pub mod schema;
pub mod value;

mod parse;
pub mod types;

pub use error::{ParseError, ValueAccessError};
pub use parse::{parse, parse_schema, parse_with_schema, parse_with_schema_str};
pub use schema::{Schema, SchemaEntry};
pub use tycl_macro::TryFromValue;
pub use value::{Document, TimeValue, TryFromValue, Value};

#[cfg(feature = "serde")]
mod serde_impl;

#[cfg(feature = "serde")]
pub use serde_impl::{SerdeError, from_document, from_value, to_document, to_value};

/// Parse a TyCL document from a string.
pub fn from_str(source: &str) -> Result<Document, ParseError> {
    parse(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn parse_simple_document() {
        let source = r#"
name = "hello"
count = 42
items = [1, 2, 3]
"#;
        let doc = parse(source).unwrap();
        assert_eq!(
            doc.root.get("name"),
            Some(&Value::String("hello".to_string()))
        );
        assert_eq!(doc.root.get("count"), Some(&Value::Integer(42)));
        assert!(matches!(doc.root.get("items"), Some(Value::List(_))));
    }

    #[test]
    fn parse_map_and_tuple() {
        let source = r#"
config = { a = 1, b = "two" }
pair = (true, null)
"#;
        let doc = parse(source).unwrap();
        assert!(matches!(doc.root.get("config"), Some(Value::Map(_))));
        assert!(matches!(doc.root.get("pair"), Some(Value::Tuple(_))));
    }

    #[test]
    fn parse_time_values() {
        let source = r#"
d = 2024-05-20
t = 10:30:00
dt = 2024-05-20T10:30:00
odt = 2024-05-20T10:30:00Z
"#;
        let doc = parse(source).unwrap();
        assert_eq!(
            doc.root.get("d"),
            Some(&Value::Time(TimeValue::LocalDate("2024-05-20".to_string())))
        );
        assert_eq!(
            doc.root.get("t"),
            Some(&Value::Time(TimeValue::LocalTime("10:30:00".to_string())))
        );
        assert_eq!(
            doc.root.get("dt"),
            Some(&Value::Time(TimeValue::LocalDateTime(
                "2024-05-20T10:30:00".to_string()
            )))
        );
        assert_eq!(
            doc.root.get("odt"),
            Some(&Value::Time(TimeValue::OffsetDateTime(
                "2024-05-20T10:30:00Z".to_string()
            )))
        );
    }

    #[test]
    fn duplicate_key_errors() {
        let source = "a = 1\na = 2";
        assert!(parse(source).is_err());
    }

    #[test]
    fn parse_type_annotations() {
        let source = r#"
a: int = 1
b: str = "hello"
c: bool = true
"#;
        let doc = parse(source).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(1)));
        assert_eq!(doc.root.get("b"), Some(&Value::String("hello".to_string())));
        assert_eq!(doc.root.get("c"), Some(&Value::Bool(true)));
    }

    #[test]
    fn parse_nullable_type() {
        let source = r#"
a: str? = null
b: int? = 42
"#;
        let doc = parse(source).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Null));
        assert_eq!(doc.root.get("b"), Some(&Value::Integer(42)));
    }

    #[test]
    fn type_mismatch_errors() {
        assert!(parse("a: int = \"x\"").is_err());
        assert!(parse("a: int = null").is_err());
        assert!(parse("a: str? = 1").is_err());
    }

    #[test]
    fn parse_list_with_type() {
        let source = r#"items: list(int) = [1, 2, 3]"#;
        let doc = parse(source).unwrap();
        assert!(matches!(doc.root.get("items"), Some(Value::List(_))));
    }

    #[test]
    fn list_type_mismatch() {
        assert!(parse("items: list(int) = [1, \"x\"]").is_err());
    }

    #[test]
    fn parse_map_with_type() {
        let source = r#"m: map(str) = { a = "x", b = "y" }"#;
        let doc = parse(source).unwrap();
        assert!(matches!(doc.root.get("m"), Some(Value::Map(_))));
    }

    #[test]
    fn parse_tuple_with_type() {
        let source = r#"t: tuple(int, str) = (1, "x")"#;
        let doc = parse(source).unwrap();
        assert!(matches!(doc.root.get("t"), Some(Value::Tuple(_))));
    }

    #[test]
    fn tuple_arity_mismatch() {
        assert!(parse("t: tuple(int, str) = (1, \"x\", 3)").is_err());
        assert!(parse("t: tuple(int, str) = (1)").is_err());
    }

    #[test]
    fn parse_record_type() {
        let source = r#"r: record(a: int, b: str) = { a = 1, b = "x" }"#;
        let doc = parse(source).unwrap();
        assert!(matches!(doc.root.get("r"), Some(Value::Record(_))));
    }

    #[test]
    fn open_record_allows_extra_fields() {
        let source = r#"r: record(a: int, ...) = { a = 1, b = "x" }"#;
        let doc = parse(source).unwrap();
        assert!(matches!(doc.root.get("r"), Some(Value::Record(_))));
    }

    #[test]
    fn closed_record_rejects_extra_fields() {
        assert!(parse("r: record(a: int) = { a = 1, b = \"x\" }").is_err());
    }

    #[test]
    fn record_missing_field_errors() {
        assert!(parse("r: record(a: int) = { b = 1 }").is_err());
    }

    #[test]
    fn subtyping_with_any() {
        let source = r#"a: list(any) = [1, "x"]"#;
        let doc = parse(source).unwrap();
        assert!(matches!(doc.root.get("a"), Some(Value::List(_))));
    }

    #[test]
    fn pure_inference_list_any() {
        let source = r#"value = [ (1, "2"), (3) ]"#;
        let doc = parse(source).unwrap();
        assert!(matches!(doc.root.get("value"), Some(Value::List(_))));
    }

    // ================================================================
    // API tests
    // ================================================================

    #[test]
    fn from_str_works() {
        let doc = from_str("a = 1").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(1)));
    }

    #[test]
    fn value_is_xxx() {
        assert!(Value::Null.is_null());
        assert!(Value::Bool(true).is_bool());
        assert!(Value::Integer(1).is_integer());
        assert!(Value::Float(1.0).is_float());
        assert!(Value::String("s".into()).is_string());
        assert!(Value::Time(TimeValue::LocalDate("d".into())).is_time());
        assert!(Value::List(vec![]).is_list());
        assert!(Value::Tuple(vec![]).is_list());
        assert!(Value::Map(BTreeMap::new()).is_map());
        assert!(Value::Record(BTreeMap::new()).is_map());
        assert!(Value::Record(BTreeMap::new()).is_record());
        assert!(Value::Tuple(vec![]).is_tuple());
    }

    #[test]
    fn as_xxx_null_returns_none() {
        let v = Value::Null;
        assert_eq!(v.as_bool().unwrap(), None);
        assert_eq!(v.as_integer().unwrap(), None);
        assert_eq!(v.as_string().unwrap(), None);
        assert_eq!(v.as_list().unwrap(), None);
        assert_eq!(v.as_map().unwrap(), None);
        assert_eq!(v.as_record().unwrap(), None);
        assert_eq!(v.as_tuple().unwrap(), None);
    }

    #[test]
    fn as_xxx_mismatch_returns_err() {
        let v = Value::Integer(1);
        assert!(v.as_bool().is_err());
        assert!(v.as_string().is_err());
        assert!(v.as_record().is_err());
        assert!(v.as_tuple().is_err());
    }

    #[test]
    fn as_map_accepts_map_and_record() {
        let m = Value::Map(BTreeMap::new());
        let r = Value::Record(BTreeMap::new());
        assert!(m.as_map().unwrap().is_some());
        assert!(r.as_map().unwrap().is_some());
    }

    #[test]
    fn as_record_rejects_map() {
        let m = Value::Map(BTreeMap::new());
        assert!(m.as_record().is_err());
    }

    #[test]
    fn as_list_accepts_list_and_tuple() {
        let l = Value::List(vec![]);
        let t = Value::Tuple(vec![]);
        assert!(l.as_list().unwrap().is_some());
        assert!(t.as_list().unwrap().is_some());
    }

    #[test]
    fn as_tuple_rejects_list() {
        let l = Value::List(vec![]);
        assert!(l.as_tuple().is_err());
    }

    #[test]
    fn document_indexing() {
        let doc = parse(r#"a = 1"#).unwrap();
        assert_eq!(doc["a"], Value::Integer(1));
    }

    #[test]
    fn value_map_indexing() {
        let doc = parse(r#"m = { a = 1, b = 2 }"#).unwrap();
        let m = doc.root.get("m").unwrap();
        assert_eq!(m["a"], Value::Integer(1));
        assert_eq!(m["b"], Value::Integer(2));
    }

    #[test]
    fn value_list_indexing() {
        let doc = parse(r#"items = [1, 2, 3]"#).unwrap();
        let items = doc.root.get("items").unwrap();
        assert_eq!(items[0], Value::Integer(1));
        assert_eq!(items[2], Value::Integer(3));
    }

    #[test]
    fn try_into_primitives() {
        let v = Value::Integer(42);
        let n: i64 = v.try_into().unwrap();
        assert_eq!(n, 42);

        let v = Value::String("hello".into());
        let s: String = v.try_into().unwrap();
        assert_eq!(s, "hello");
    }

    #[test]
    fn try_into_list_generic() {
        let v = Value::List(vec![Value::Integer(1), Value::Integer(2)]);
        let list: Vec<i64> = Vec::try_from_value(v).unwrap();
        assert_eq!(list, vec![1, 2]);
    }

    #[test]
    fn try_into_map_generic() {
        let mut map = BTreeMap::new();
        map.insert("a".into(), Value::Integer(1));
        let v = Value::Map(map);
        let m: std::collections::BTreeMap<String, i64> = BTreeMap::try_from_value(v).unwrap();
        assert_eq!(m.get("a"), Some(&1));
    }

    #[test]
    fn try_into_option() {
        let v = Value::Null;
        let o: Option<i64> = Option::try_from_value(v).unwrap();
        assert_eq!(o, None);

        let v = Value::Integer(1);
        let o: Option<i64> = Option::try_from_value(v).unwrap();
        assert_eq!(o, Some(1));
    }

    #[test]
    fn as_list_t_converts_elements() {
        let v = Value::List(vec![Value::Integer(1), Value::Integer(2)]);
        let list = v.as_list_t::<i64>().unwrap().unwrap();
        assert_eq!(list, vec![1, 2]);
    }

    #[test]
    fn as_map_t_converts_values() {
        let mut map = BTreeMap::new();
        map.insert("x".into(), Value::Integer(10));
        let v = Value::Map(map);
        let m = v.as_map_t::<i64>().unwrap().unwrap();
        assert_eq!(m.get("x"), Some(&10));
    }

    #[test]
    fn as_record_t_only_accepts_record() {
        let map = Value::Map(BTreeMap::new());
        assert!(map.as_record_t::<BTreeMap<String, Value>>().is_err());

        let rec = Value::Record(BTreeMap::new());
        assert!(
            rec.as_record_t::<BTreeMap<String, Value>>()
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn as_tuple_t_only_accepts_tuple() {
        let list = Value::List(vec![]);
        assert!(list.as_tuple_t::<Vec<Value>>().is_err());

        let tup = Value::Tuple(vec![]);
        assert!(tup.as_tuple_t::<Vec<Value>>().unwrap().is_some());
    }

    // ================================================================
    // Derive macro tests
    // ================================================================

    #[derive(TryFromValue, Debug, PartialEq)]
    struct Person {
        name: String,
        age: i64,
    }

    #[test]
    fn derive_named_struct_from_record() {
        let mut map = BTreeMap::new();
        map.insert("name".into(), Value::String("Alice".into()));
        map.insert("age".into(), Value::Integer(30));
        let value = Value::Record(map);

        let person = Person::try_from_value(value).unwrap();
        assert_eq!(
            person,
            Person {
                name: "Alice".into(),
                age: 30,
            }
        );
    }

    #[test]
    fn derive_named_struct_rejects_map() {
        let mut map = BTreeMap::new();
        map.insert("name".into(), Value::String("Alice".into()));
        map.insert("age".into(), Value::Integer(30));
        let value = Value::Map(map);

        assert!(Person::try_from_value(value).is_err());
    }

    #[test]
    fn derive_named_struct_missing_field() {
        let mut map = BTreeMap::new();
        map.insert("name".into(), Value::String("Alice".into()));
        let value = Value::Record(map);

        assert!(Person::try_from_value(value).is_err());
    }

    #[derive(TryFromValue, Debug, PartialEq)]
    struct Config {
        enabled: bool,
        timeout: Option<i64>,
    }

    #[test]
    fn derive_named_struct_with_option_field() {
        let mut map = BTreeMap::new();
        map.insert("enabled".into(), Value::Bool(true));
        map.insert("timeout".into(), Value::Integer(10));
        let value = Value::Record(map);

        let cfg = Config::try_from_value(value).unwrap();
        assert_eq!(
            cfg,
            Config {
                enabled: true,
                timeout: Some(10),
            }
        );
    }

    #[test]
    fn derive_named_struct_option_none_when_missing() {
        let mut map = BTreeMap::new();
        map.insert("enabled".into(), Value::Bool(false));
        let value = Value::Record(map);

        let cfg = Config::try_from_value(value).unwrap();
        assert_eq!(
            cfg,
            Config {
                enabled: false,
                timeout: None,
            }
        );
    }

    #[derive(TryFromValue, Debug, PartialEq)]
    struct Point(f64, f64);

    #[test]
    fn derive_tuple_struct_from_tuple() {
        let value = Value::Tuple(vec![Value::Float(1.0), Value::Float(2.0)]);
        let point = Point::try_from_value(value).unwrap();
        assert_eq!(point, Point(1.0, 2.0));
    }

    #[test]
    fn derive_tuple_struct_rejects_list() {
        let value = Value::List(vec![Value::Float(1.0), Value::Float(2.0)]);
        assert!(Point::try_from_value(value).is_err());
    }

    #[test]
    fn derive_tuple_struct_index_out_of_bounds() {
        let value = Value::Tuple(vec![Value::Float(1.0)]);
        assert!(Point::try_from_value(value).is_err());
    }

    #[derive(TryFromValue, Debug, PartialEq)]
    struct Tagged(String, Option<i64>);

    #[test]
    fn derive_tuple_struct_with_option() {
        let value = Value::Tuple(vec![Value::String("tag".into()), Value::Null]);
        let tagged = Tagged::try_from_value(value).unwrap();
        assert_eq!(tagged, Tagged("tag".into(), None));
    }

    // ================================================================
    // Schema tests
    // ================================================================

    #[test]
    fn schema_self_validation_error() {
        assert!(parse_schema("a: int = \"x\"").is_err());
    }

    #[test]
    fn data_target_name_in_non_schema_fails() {
        assert!(parse("a (rename) = 1").is_err());
    }

    #[test]
    fn parse_with_schema_basic_validation() {
        let schema = parse_schema("a: int = 0").unwrap();
        assert!(parse_with_schema("a = 1", &schema).is_ok());
        assert!(parse_with_schema("a = \"x\"", &schema).is_err());
    }

    #[test]
    fn parse_with_schema_top_level_default() {
        let schema = parse_schema("a: int = 42\nb: str = \"hello\"").unwrap();
        let doc = parse_with_schema("a = 1", &schema).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(1)));
        assert_eq!(doc.root.get("b"), Some(&Value::String("hello".into())));
    }

    #[test]
    fn parse_with_schema_record_default() {
        let schema =
            parse_schema(r#"r: record(a: int, b: str = "default") = { a = 0, b = "default" }"#)
                .unwrap();
        let doc = parse_with_schema("r = { a = 1 }", &schema).unwrap();
        let rec = doc.root.get("r").unwrap().as_record().unwrap().unwrap();
        assert_eq!(rec.get("a"), Some(&Value::Integer(1)));
        assert_eq!(rec.get("b"), Some(&Value::String("default".into())));
    }

    #[test]
    fn parse_with_schema_top_level_rename() {
        let schema = parse_schema("tycl_key (renamed): int = 0").unwrap();
        let doc = parse_with_schema("tycl_key = 42", &schema).unwrap();
        assert_eq!(doc.root.get("renamed"), Some(&Value::Integer(42)));
        assert!(doc.root.get("tycl_key").is_none());
    }

    #[test]
    fn parse_with_schema_record_rename() {
        let schema =
            parse_schema(r#"r: record(tycl_field (out_field): int = 0) = { tycl_field = 0 }"#)
                .unwrap();
        let doc = parse_with_schema("r = { tycl_field = 99 }", &schema).unwrap();
        let rec = doc.root.get("r").unwrap().as_record().unwrap().unwrap();
        assert_eq!(rec.get("out_field"), Some(&Value::Integer(99)));
        assert!(rec.get("tycl_field").is_none());
    }

    #[test]
    fn parse_with_schema_unknown_key_allowed() {
        let schema = parse_schema("a: int = 0").unwrap();
        let doc = parse_with_schema("a = 1\nunknown = \"ok\"", &schema).is_ok();
        assert!(doc);
    }

    #[test]
    fn parse_with_schema_str_convenience() {
        let doc = parse_with_schema_str("x = 1", "x: int = 0").unwrap();
        assert_eq!(doc.root.get("x"), Some(&Value::Integer(1)));
    }

    // ================================================================
    // Lexer / String edge cases
    // ================================================================

    #[test]
    fn invalid_escape_in_basic_string() {
        assert!(parse(r#"a = "abc\q""#).is_err());
    }

    #[test]
    fn unterminated_basic_string() {
        assert!(parse("a = \"abc").is_err());
    }

    #[test]
    fn unterminated_ml_string() {
        assert!(parse(r#"a = """abc"#).is_err());
    }

    #[test]
    fn unterminated_raw_string() {
        assert!(parse(r#"a = r#"abc"#).is_err());
    }

    #[test]
    fn unterminated_aligned_ml_string() {
        assert!(parse("a = |\"\"\"\nabc").is_err());
    }

    #[test]
    fn unterminated_aligned_raw_string() {
        assert!(parse(r#"a = r|#"abc"#).is_err());
    }

    #[test]
    fn invalid_raw_string_delimiter() {
        // Opening has two hashes but closing has one
        assert!(parse(r##"a = r##"abc"#"##).is_err());
    }

    #[test]
    fn missing_alignment_trigger() {
        let q = r#"""""#; // produces """
        let source = format!("a = |{q}\n|line1\nno_trigger\n|{q}");
        assert!(parse(&source).is_err());
    }

    #[test]
    fn invalid_integer_literal() {
        // Value far beyond i64 range should fail lexing
        assert!(parse("a = 999999999999999999999999999999").is_err());
    }

    #[test]
    fn invalid_float_literal() {
        assert!(parse("a = 1.0e309").is_err());
    }

    #[test]
    fn basic_string_empty() {
        let doc = parse(r#"a = """#).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::String("".into())));
    }

    #[test]
    fn basic_string_escapes() {
        let doc = parse(r#"a = "\b\f\n\r\t\\\'\"""#).unwrap();
        assert_eq!(
            doc.root.get("a"),
            Some(&Value::String("\x08\x0C\n\r\t\\'\"".into()))
        );
    }

    #[test]
    fn basic_string_unicode_escape_4() {
        let doc = parse(r#"a = "\u0041\u0042""#).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::String("AB".into())));
    }

    #[test]
    fn basic_string_unicode_escape_8() {
        let doc = parse(r#"a = "\U00000041\U00000042""#).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::String("AB".into())));
    }

    #[test]
    fn basic_string_invalid_unicode_short() {
        assert!(parse(r#"a = "\u41""#).is_err());
    }

    #[test]
    fn ml_string_empty() {
        let doc = parse(r#"a = """""""#).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::String("".into())));
    }

    #[test]
    fn ml_string_with_initial_newline() {
        let doc = parse(
            r##"a = """
content
""""##,
        )
        .unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::String("content\n".into())));
    }

    #[test]
    fn ml_string_without_initial_newline() {
        let doc = parse(r#"a = """content""""#).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::String("content".into())));
    }

    #[test]
    fn raw_string_with_quotes_inside() {
        let doc = parse(r##"a = r#"a"b"#"##).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::String(r#"a"b"#.into())));
    }

    #[test]
    fn raw_string_empty() {
        let doc = parse(r##"a = r#""#"##).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::String("".into())));
    }

    #[test]
    fn aligned_ml_string_empty() {
        let source = "a = |\"\"\"\n|\"\"\"";
        let doc = parse(source).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::String("".into())));
    }

    #[test]
    fn aligned_ml_string_with_content() {
        let source = "a = |\"\"\"\n|  line1\n|  line2\n|\"\"\"";
        let doc = parse(source).unwrap();
        assert_eq!(
            doc.root.get("a"),
            Some(&Value::String("  line1\n  line2".into()))
        );
    }

    #[test]
    fn aligned_raw_string_with_quotes() {
        let source = "a = r|#\"\n|a\"b\n|#\"";
        let doc = parse(source).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::String("a\"b".into())));
    }

    // ================================================================
    // Number edge cases
    // ================================================================

    #[test]
    fn integer_with_plus_sign() {
        let doc = parse("a = +42").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(42)));
    }

    #[test]
    fn integer_leading_zeros() {
        let doc = parse("a = 007").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(7)));
    }

    #[test]
    fn integer_with_underscores() {
        let doc = parse("a = 1_000_000").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(1_000_000)));
    }

    #[test]
    fn integer_min_i64() {
        let doc = parse("a = -9223372036854775808").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(i64::MIN)));
    }

    #[test]
    fn integer_max_i64() {
        let doc = parse("a = 9223372036854775807").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(i64::MAX)));
    }

    #[test]
    fn float_leading_dot() {
        let doc = parse("a = .5").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Float(0.5)));
    }

    #[test]
    fn float_trailing_dot_rejected() {
        // Trailing dot without digits is rejected to avoid ambiguity with `..` in ranges.
        assert!(parse("a = 5.").is_err());
    }

    #[test]
    fn float_exp_lowercase() {
        let doc = parse("a = 5e10").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Float(5e10)));
    }

    #[test]
    fn float_exp_uppercase_negative() {
        let doc = parse("a = 5E-10").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Float(5e-10)));
    }

    #[test]
    fn float_leading_dot_with_exp() {
        let doc = parse("a = .5e+2").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Float(50.0)));
    }

    #[test]
    fn float_with_underscores() {
        let doc = parse("a = 1_000.000_001").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Float(1000.000001)));
    }

    // ================================================================
    // Key / whitespace / comment edge cases
    // ================================================================

    #[test]
    fn builtin_key() {
        let doc = parse(r#"$env = "prod""#).unwrap();
        assert_eq!(doc.root.get("$env"), Some(&Value::String("prod".into())));
    }

    #[test]
    fn identifier_with_hyphen() {
        let doc = parse("my-key = 1").unwrap();
        assert_eq!(doc.root.get("my-key"), Some(&Value::Integer(1)));
    }

    #[test]
    fn identifier_with_underscore() {
        let doc = parse("my_key = 1").unwrap();
        assert_eq!(doc.root.get("my_key"), Some(&Value::Integer(1)));
    }

    #[test]
    fn literal_key_basic() {
        let doc = parse(r#""spaced key" = 1"#).unwrap();
        assert_eq!(doc.root.get("spaced key"), Some(&Value::Integer(1)));
    }

    #[test]
    fn literal_key_with_escape() {
        let doc = parse(
            r#""key
" = 1"#,
        )
        .unwrap();
        assert_eq!(doc.root.get("key\n"), Some(&Value::Integer(1)));
    }

    #[test]
    fn comment_between_kv_pairs() {
        let doc = parse("a = 1 // c\nb = 2").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(1)));
        assert_eq!(doc.root.get("b"), Some(&Value::Integer(2)));
    }

    #[test]
    fn trailing_comment() {
        let doc = parse("a = 1 // trailing").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(1)));
    }

    #[test]
    fn empty_lines_between_pairs() {
        let doc = parse("a = 1\n\nb = 2").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(1)));
        assert_eq!(doc.root.get("b"), Some(&Value::Integer(2)));
    }

    #[test]
    fn tabs_vs_spaces() {
        let doc = parse("\ta\t=\t1").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(1)));
    }

    #[test]
    fn mixed_crlf_lf() {
        let source = "a = 1\r\nb = 2\nc = 3";
        let doc = parse(source).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(1)));
        assert_eq!(doc.root.get("b"), Some(&Value::Integer(2)));
        assert_eq!(doc.root.get("c"), Some(&Value::Integer(3)));
    }

    #[test]
    fn bare_cr_newline_enforcement() {
        // After fix, bare CR should count as newline separator
        assert!(parse("a = 1\rb = 2").is_ok());
    }

    // ================================================================
    // Container edge cases
    // ================================================================

    #[test]
    fn deeply_nested_containers() {
        let source = r#"a = { b = [ ( { c = 1 }, [2, 3] ) ] }"#;
        let doc = parse(source).unwrap();
        assert!(matches!(doc.root.get("a"), Some(Value::Map(_))));
    }

    #[test]
    fn empty_list() {
        let doc = parse("a = []").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::List(vec![])));
    }

    #[test]
    fn empty_map() {
        let doc = parse("a = {}").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Map(BTreeMap::new())));
    }

    #[test]
    fn empty_tuple() {
        let doc = parse("a = ()").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Tuple(vec![])));
    }

    #[test]
    fn trailing_comma_in_list() {
        let doc = parse("a = [1, 2,]").unwrap();
        assert_eq!(
            doc.root.get("a"),
            Some(&Value::List(vec![Value::Integer(1), Value::Integer(2)]))
        );
    }

    #[test]
    fn trailing_comma_in_map() {
        let doc = parse(r#"a = { a = 1, b = 2, }"#).unwrap();
        let mut expected = BTreeMap::new();
        expected.insert("a".into(), Value::Integer(1));
        expected.insert("b".into(), Value::Integer(2));
        assert_eq!(doc.root.get("a"), Some(&Value::Map(expected)));
    }

    #[test]
    fn trailing_comma_in_tuple() {
        let doc = parse(r#"a = (1, 2,)"#).unwrap();
        assert_eq!(
            doc.root.get("a"),
            Some(&Value::Tuple(vec![Value::Integer(1), Value::Integer(2)]))
        );
    }

    #[test]
    fn duplicate_key_in_map_errors() {
        assert!(parse(r#"a = { b = 1, b = 2 }"#).is_err());
    }

    #[test]
    fn tuple_inside_tuple() {
        let doc = parse(r#"a = ((1, 2), (3, 4))"#).unwrap();
        let inner = Value::Tuple(vec![Value::Integer(1), Value::Integer(2)]);
        assert_eq!(
            doc.root.get("a"),
            Some(&Value::Tuple(vec![
                inner,
                Value::Tuple(vec![Value::Integer(3), Value::Integer(4)])
            ]))
        );
    }

    #[test]
    fn list_inside_list() {
        let doc = parse(r#"a = [[1, 2], [3, 4]]"#).unwrap();
        assert_eq!(
            doc.root.get("a"),
            Some(&Value::List(vec![
                Value::List(vec![Value::Integer(1), Value::Integer(2)]),
                Value::List(vec![Value::Integer(3), Value::Integer(4)])
            ]))
        );
    }

    // ================================================================
    // Newline enforcement edge cases
    // ================================================================

    #[test]
    fn missing_newline_between_top_level_pairs() {
        assert!(parse("a = 1 b = 2").is_err());
    }

    // ================================================================
    // Type system edge cases
    // ================================================================

    #[test]
    fn str_pattern_constraint_parses() {
        let doc = parse(r#"a: str("^[a-z]+$") = "hello""#).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::String("hello".into())));
    }

    #[test]
    fn str_pattern_constraint_mismatch() {
        assert!(parse(r#"a: str("^[a-z]+$") = "Hello123""#).is_err());
    }

    #[test]
    fn int_range_constraint_parses() {
        use crate::lexer::Token;
        use logos::Logos;
        let s = "a: int(0..100) = 50";
        let mut lex = Token::lexer(s);
        while let Some(tok) = lex.next() {
            eprintln!("{:?} {:?}", lex.span(), tok);
        }
        let doc = parse(s).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(50)));
    }

    #[test]
    fn int_range_constraint_out_of_range() {
        assert!(parse("a: int(0..100) = 150").is_err());
    }

    #[test]
    fn float_range_constraint_parses() {
        let doc = parse("a: float(0.0..1.0) = 0.5").unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Float(0.5)));
    }

    #[test]
    fn float_range_constraint_out_of_range() {
        assert!(parse("a: float(0.0..1.0) = 1.5").is_err());
    }

    #[test]
    fn time_subtype_match_localdate() {
        let doc = parse("a: time(localdate) = 2024-05-20").unwrap();
        assert_eq!(
            doc.root.get("a"),
            Some(&Value::Time(TimeValue::LocalDate("2024-05-20".into())))
        );
    }

    #[test]
    fn time_subtype_mismatch() {
        assert!(parse("a: time(localdate) = 10:30:00").is_err());
    }

    #[test]
    fn nullable_any_vs_non_nullable_any() {
        assert!(parse("a: any = null").is_err());
        assert_eq!(
            parse("a: any? = null").unwrap().root.get("a"),
            Some(&Value::Null)
        );
        assert_eq!(
            parse("a: any = 1").unwrap().root.get("a"),
            Some(&Value::Integer(1))
        );
    }

    #[test]
    fn nested_map_list_type() {
        let doc = parse(r#"a: map(list(int)) = { x = [1, 2] }"#).unwrap();
        assert!(matches!(doc.root.get("a"), Some(Value::Map(_))));
    }

    #[test]
    fn enum_type_match_valid_value() {
        let doc = parse(r#"a: enum("a", "b") = "a""#).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::String("a".into())));
    }

    #[test]
    fn enum_type_reject_invalid_value() {
        assert!(parse(r#"a: enum("a", "b") = "c""#).is_err());
    }

    #[test]
    fn env_type_with_fallback_match() {
        let doc = parse(r#"a: env("VAR", int) = 42"#).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::Integer(42)));
    }

    #[test]
    fn env_type_without_fallback() {
        let doc = parse(r#"a: env("VAR") = "x""#).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::String("x".into())));
    }

    // ================================================================
    // Schema edge cases
    // ================================================================

    #[test]
    fn schema_enum_with_target_name() {
        let schema = parse_schema(r#"a: enum("x" (out_x), "y") = "x""#).unwrap();
        let doc = parse_with_schema(r#"a = "x""#, &schema).unwrap();
        assert_eq!(doc.root.get("out_x"), Some(&Value::String("x".into())));
    }

    #[test]
    fn schema_record_field_target_name_and_default() {
        let schema = parse_schema(r#"r: record(f (out_f): int = 42) = { f = 0 }"#).unwrap();
        let doc = parse_with_schema("r = {}", &schema).unwrap();
        let rec = doc.root.get("r").unwrap().as_record().unwrap().unwrap();
        assert_eq!(rec.get("out_f"), Some(&Value::Integer(42)));
    }

    #[test]
    fn schema_empty_target_name() {
        let schema = parse_schema("a (): int = 1").unwrap();
        let doc = parse_with_schema("a = 2", &schema).unwrap();
        assert_eq!(doc.root.get(""), Some(&Value::Integer(2)));
    }

    #[test]
    fn schema_nested_target_name_in_record() {
        let schema = parse_schema(
            r#"r: record(inner (out_inner): record(f (out_f): int = 0) = { f = 0 }) = { inner = { f = 0 } }"#,
        )
        .unwrap();
        let doc = parse_with_schema("r = { inner = { f = 99 } }", &schema).unwrap();
        let rec = doc.root.get("r").unwrap().as_record().unwrap().unwrap();
        let inner = rec.get("out_inner").unwrap().as_record().unwrap().unwrap();
        assert_eq!(inner.get("out_f"), Some(&Value::Integer(99)));
    }

    #[test]
    fn schema_with_env_type() {
        let schema = parse_schema(r#"a: env("HOME", str) = "default""#).unwrap();
        let doc = parse_with_schema(r#"a = "ok""#, &schema).unwrap();
        assert_eq!(doc.root.get("a"), Some(&Value::String("ok".into())));
    }

    // ================================================================
    // Serde tests
    // ================================================================

    #[cfg(feature = "serde")]
    mod serde_tests {
        use super::*;
        use crate::{from_document, from_value, to_document, to_value};
        use serde::{Deserialize, Serialize};

        #[test]
        fn serde_bool_roundtrip() {
            let v = to_value(true).unwrap();
            assert_eq!(v, Value::Bool(true));
            assert_eq!(from_value::<bool>(v).unwrap(), true);
        }

        #[test]
        fn serde_integer_roundtrip() {
            let v = to_value(42i64).unwrap();
            assert_eq!(v, Value::Integer(42));
            assert_eq!(from_value::<i64>(v).unwrap(), 42);
        }

        #[test]
        fn serde_float_roundtrip() {
            let v = to_value(3.14f64).unwrap();
            assert_eq!(v, Value::Float(3.14));
            assert_eq!(from_value::<f64>(v).unwrap(), 3.14);
        }

        #[test]
        fn serde_string_roundtrip() {
            let v = to_value("hello".to_string()).unwrap();
            assert_eq!(v, Value::String("hello".into()));
            assert_eq!(from_value::<String>(v).unwrap(), "hello");
        }

        #[test]
        fn serde_option_some_roundtrip() {
            let v = to_value(Some(42i64)).unwrap();
            assert_eq!(v, Value::Integer(42));
            assert_eq!(from_value::<Option<i64>>(v).unwrap(), Some(42));
        }

        #[test]
        fn serde_option_none_roundtrip() {
            let v = to_value(None::<i64>).unwrap();
            assert_eq!(v, Value::Null);
            assert_eq!(from_value::<Option<i64>>(v).unwrap(), None);
        }

        #[test]
        fn serde_vec_roundtrip() {
            let v = to_value(vec![1i64, 2, 3]).unwrap();
            assert_eq!(
                v,
                Value::List(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3)
                ])
            );
            assert_eq!(from_value::<Vec<i64>>(v).unwrap(), vec![1, 2, 3]);
        }

        #[test]
        fn serde_map_roundtrip() {
            let mut map = BTreeMap::new();
            map.insert("a".to_string(), 1i64);
            map.insert("b".to_string(), 2i64);
            let v = to_value(map).unwrap();
            let mut expected = BTreeMap::new();
            expected.insert("a".to_string(), Value::Integer(1));
            expected.insert("b".to_string(), Value::Integer(2));
            assert_eq!(v, Value::Map(expected));
            let recovered: BTreeMap<String, i64> = from_value(v).unwrap();
            assert_eq!(recovered.get("a"), Some(&1));
            assert_eq!(recovered.get("b"), Some(&2));
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct SerdePerson {
            name: String,
            age: i64,
        }

        #[test]
        fn serde_struct_roundtrip() {
            let person = SerdePerson {
                name: "Alice".into(),
                age: 30,
            };
            let v = to_value(&person).unwrap();
            let mut expected = BTreeMap::new();
            expected.insert("name".to_string(), Value::String("Alice".into()));
            expected.insert("age".to_string(), Value::Integer(30));
            assert_eq!(v, Value::Record(expected));
            let recovered: SerdePerson = from_value(v).unwrap();
            assert_eq!(recovered, person);
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct SerdePoint(f64, f64);

        #[test]
        fn serde_tuple_struct_roundtrip() {
            let point = SerdePoint(1.0, 2.0);
            let v = to_value(&point).unwrap();
            assert_eq!(v, Value::Tuple(vec![Value::Float(1.0), Value::Float(2.0)]));
            let recovered: SerdePoint = from_value(v).unwrap();
            assert_eq!(recovered, point);
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        enum SerdeColor {
            Red,
            Green,
            Rgb(u8, u8, u8),
            Named { name: String },
        }

        #[test]
        fn serde_enum_unit_variant_roundtrip() {
            let v = to_value(SerdeColor::Red).unwrap();
            assert_eq!(v, Value::String("Red".into()));
            let recovered: SerdeColor = from_value(v).unwrap();
            assert_eq!(recovered, SerdeColor::Red);
        }

        #[test]
        fn serde_enum_tuple_variant_roundtrip() {
            let v = to_value(SerdeColor::Rgb(1, 2, 3)).unwrap();
            let mut map = BTreeMap::new();
            map.insert(
                "Rgb".to_string(),
                Value::Tuple(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                ]),
            );
            assert_eq!(v, Value::Record(map));
            let recovered: SerdeColor = from_value(v).unwrap();
            assert_eq!(recovered, SerdeColor::Rgb(1, 2, 3));
        }

        #[test]
        fn serde_enum_struct_variant_roundtrip() {
            let v = to_value(SerdeColor::Named {
                name: "blue".into(),
            })
            .unwrap();
            let mut inner = BTreeMap::new();
            inner.insert("name".to_string(), Value::String("blue".into()));
            let mut map = BTreeMap::new();
            map.insert("Named".to_string(), Value::Record(inner));
            assert_eq!(v, Value::Record(map));
            let recovered: SerdeColor = from_value(v).unwrap();
            assert_eq!(
                recovered,
                SerdeColor::Named {
                    name: "blue".into()
                }
            );
        }

        #[test]
        fn serde_document_roundtrip() {
            let mut map = BTreeMap::new();
            map.insert("a".to_string(), Value::Integer(1));
            map.insert("b".to_string(), Value::String("hello".into()));
            let doc = Document { root: map };
            let recovered: Document = from_document(to_document(&doc).unwrap()).unwrap();
            assert_eq!(recovered.root.get("a"), Some(&Value::Integer(1)));
            assert_eq!(
                recovered.root.get("b"),
                Some(&Value::String("hello".into()))
            );
        }

        #[test]
        fn serde_from_document_to_struct() {
            let mut map = BTreeMap::new();
            map.insert("name".to_string(), Value::String("Bob".into()));
            map.insert("age".to_string(), Value::Integer(25));
            let doc = Document { root: map };
            let person: SerdePerson = from_document(doc).unwrap();
            assert_eq!(
                person,
                SerdePerson {
                    name: "Bob".into(),
                    age: 25,
                }
            );
        }

        #[test]
        fn serde_u64_within_i64_range() {
            let v = to_value(42u64).unwrap();
            assert_eq!(v, Value::Integer(42));
            assert_eq!(from_value::<u64>(v).unwrap(), 42);
        }

        #[test]
        fn serde_u64_out_of_i64_range() {
            assert!(to_value(u64::MAX).is_err());
            let v = Value::Integer(i64::MAX);
            assert_eq!(from_value::<u64>(v).unwrap(), i64::MAX as u64);
        }

        #[test]
        fn serde_time_localdate_roundtrip() {
            let tv = TimeValue::LocalDate("2024-05-20".into());
            let v = Value::Time(tv.clone());
            let recovered: TimeValue = from_value(v).unwrap();
            assert_eq!(recovered, tv);
        }

        #[test]
        fn serde_time_localtime_roundtrip() {
            let tv = TimeValue::LocalTime("10:30:00".into());
            let v = Value::Time(tv.clone());
            let recovered: TimeValue = from_value(v).unwrap();
            assert_eq!(recovered, tv);
        }

        #[test]
        fn serde_time_localdatetime_roundtrip() {
            let tv = TimeValue::LocalDateTime("2024-05-20T10:30:00".into());
            let v = Value::Time(tv.clone());
            let recovered: TimeValue = from_value(v).unwrap();
            assert_eq!(recovered, tv);
        }

        #[test]
        fn serde_time_offsetdatetime_roundtrip() {
            let tv = TimeValue::OffsetDateTime("2024-05-20T10:30:00Z".into());
            let v = Value::Time(tv.clone());
            let recovered: TimeValue = from_value(v).unwrap();
            assert_eq!(recovered, tv);
        }
    }
}
