//! Runtime values for the Patchwork interpreter.

use std::collections::HashMap;
use std::fmt;

use serde_json::Value as JsonValue;

/// A runtime value in the Patchwork language.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// The null value.
    Null,
    /// A string value.
    String(String),
    /// A numeric value (always f64, like JavaScript).
    Number(f64),
    /// A boolean value.
    Boolean(bool),
    /// An array of values.
    Array(Vec<Value>),
    /// An object with string keys.
    Object(HashMap<String, Value>),
}

impl Value {
    /// Coerce this value to a string.
    pub fn to_string_value(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::String(s) => s.clone(),
            Value::Number(n) => {
                if n.is_nan() {
                    "NaN".to_string()
                } else if n.is_infinite() {
                    if *n > 0.0 { "Infinity" } else { "-Infinity" }.to_string()
                } else if *n == n.trunc() && n.abs() < 1e15 {
                    // Integer-like numbers without decimal point
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string_value()).collect();
                items.join(", ")
            }
            Value::Object(_) => "[object Object]".to_string(),
        }
    }

    /// Coerce this value to a boolean.
    pub fn to_bool(&self) -> bool {
        match self {
            Value::Null => false,
            Value::String(s) => !s.is_empty(),
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::Boolean(b) => *b,
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(_) => true,
        }
    }

    /// Check if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Parse a JSON string into a Value.
    pub fn from_json(s: &str) -> Result<Value, String> {
        let json: JsonValue = serde_json::from_str(s)
            .map_err(|e| format!("JSON parse error: {}", e))?;
        Ok(Value::from_json_value(json))
    }

    /// Convert a serde_json Value to our Value type.
    fn from_json_value(json: JsonValue) -> Value {
        match json {
            JsonValue::Null => Value::Null,
            JsonValue::Bool(b) => Value::Boolean(b),
            JsonValue::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
            JsonValue::String(s) => Value::String(s),
            JsonValue::Array(arr) => {
                Value::Array(arr.into_iter().map(Value::from_json_value).collect())
            }
            JsonValue::Object(obj) => {
                let map = obj.into_iter()
                    .map(|(k, v)| (k, Value::from_json_value(v)))
                    .collect();
                Value::Object(map)
            }
        }
    }

    /// Convert this Value to a JSON string.
    pub fn to_json(&self) -> String {
        let json = self.to_json_value();
        serde_json::to_string_pretty(&json).unwrap_or_else(|_| "null".to_string())
    }

    /// Convert this Value to a serde_json Value.
    fn to_json_value(&self) -> JsonValue {
        match self {
            Value::Null => JsonValue::Null,
            Value::Boolean(b) => JsonValue::Bool(*b),
            Value::Number(n) => {
                serde_json::Number::from_f64(*n)
                    .map(JsonValue::Number)
                    .unwrap_or(JsonValue::Null)
            }
            Value::String(s) => JsonValue::String(s.clone()),
            Value::Array(arr) => {
                JsonValue::Array(arr.iter().map(|v| v.to_json_value()).collect())
            }
            Value::Object(obj) => {
                let map: serde_json::Map<String, JsonValue> = obj.iter()
                    .map(|(k, v)| (k.clone(), v.to_json_value()))
                    .collect();
                JsonValue::Object(map)
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_value())
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}
