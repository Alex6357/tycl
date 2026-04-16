use crate::types::Type;
use crate::value::Value;
use std::collections::BTreeMap;

/// A parsed TyCL schema document.
///
/// Schema entries hold the expected type, optional target-name rename,
/// and default value for each top-level key.
#[derive(Clone, Debug, PartialEq)]
pub struct Schema {
    pub entries: BTreeMap<String, SchemaEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SchemaEntry {
    pub target_name: Option<String>,
    pub ty: Type,
    pub default: Value,
}
