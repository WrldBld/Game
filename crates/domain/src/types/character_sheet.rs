use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum SheetValue {
    Integer(i32),
    Float(f32),
    Boolean(bool),
    String(String),
    List(Vec<SheetValue>),
    Object(std::collections::BTreeMap<String, SheetValue>),
    Null,
}

impl SheetValue {
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            SheetValue::Integer(value) => (*value).try_into().ok(),
            SheetValue::Float(value) => (*value as i64).try_into().ok(),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            SheetValue::Integer(value) => Some(*value as i64),
            SheetValue::Float(value) => Some(*value as i64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            SheetValue::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            SheetValue::String(value) => Some(value.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSheetValues {
    pub values: std::collections::BTreeMap<String, SheetValue>,
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
}

impl CharacterSheetValues {
    pub fn get(&self, key: &str) -> Option<&SheetValue> {
        self.values.get(key)
    }

    pub fn set(&mut self, key: &str, value: SheetValue) {
        self.values.insert(key.to_string(), value);
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|value| match value {
            SheetValue::String(value) => Some(value.clone()),
            _ => None,
        })
    }

    pub fn get_number(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(SheetValue::as_i64)
    }

    pub fn get_numeric_value(&self, key: &str) -> Option<i32> {
        self.get(key).and_then(|value| match value {
            SheetValue::Integer(value) => Some(*value),
            SheetValue::Float(value) => Some(*value as i32),
            SheetValue::Boolean(value) => Some(if *value { 1 } else { 0 }),
            SheetValue::String(value) => value.parse::<i32>().ok(),
            SheetValue::Object(_) | SheetValue::List(_) | SheetValue::Null => None,
        })
    }
}
