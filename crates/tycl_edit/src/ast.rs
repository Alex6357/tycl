use crate::trivia::Trivia;
use indexmap::IndexMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TimeValue {
    LocalDate(String),
    LocalTime(String),
    LocalDateTime(String),
    OffsetDateTime(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub items: IndexMap<String, Item>,
    pub trailing_trivia: Vec<Trivia>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub key: Key,
    pub target_name: Option<String>,
    pub type_annotation: Option<String>,
    pub value: Value,
    pub leading_trivia: Vec<Trivia>,
    pub trailing_comment: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Ident(String),
    Builtin(String),
    String(String),
}

impl Key {
    pub fn as_str(&self) -> &str {
        match self {
            Key::Ident(s) | Key::Builtin(s) | Key::String(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Ident(s) => write!(f, "{}", s),
            Key::Builtin(s) => write!(f, "${}", s),
            Key::String(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Time(TimeValue),
    List(Vec<AnnotatedValue>),
    Map(Vec<(String, AnnotatedValue)>),
    Tuple(Vec<AnnotatedValue>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnnotatedValue {
    pub leading_trivia: Vec<Trivia>,
    pub value: Value,
    pub trailing_comment: Option<String>,
}

impl AnnotatedValue {
    pub fn new(value: Value) -> Self {
        Self {
            leading_trivia: Vec::new(),
            value,
            trailing_comment: None,
        }
    }
}

impl Document {
    pub fn get(&self, key: &str) -> Option<&Item> {
        self.items.get(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Item> {
        self.items.get_mut(key)
    }

    pub fn insert(&mut self, key: &str, value: Value) -> Option<Item> {
        let item = Item {
            key: Key::Ident(key.to_string()),
            target_name: None,
            type_annotation: None,
            value,
            leading_trivia: Vec::new(),
            trailing_comment: None,
        };
        self.items.insert(key.to_string(), item)
    }

    pub fn remove(&mut self, key: &str) -> Option<Item> {
        self.items.shift_remove(key)
    }
}

impl Item {
    pub fn set_value(&mut self, value: Value) {
        self.value = value;
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Integer(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Integer(v as i64)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

impl std::ops::Index<&str> for Document {
    type Output = Item;
    fn index(&self, key: &str) -> &Self::Output {
        self.items.get(key).expect("key not found")
    }
}

impl std::ops::IndexMut<&str> for Document {
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        self.items.get_mut(key).expect("key not found")
    }
}
