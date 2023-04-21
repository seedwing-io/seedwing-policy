//! Value representations in the policy engine.
//!
//! Values are inputs or outputs from patterns, and can be serialized and deserialized from different types.

use ::serde::{Deserialize, Serialize};
use indexmap::IndexMap;
use serde_json::{json, Map, Number};

use std::borrow::{Borrow, Cow};

use std::cmp::Ordering;

use std::fmt::{Debug, Display, Formatter};

use std::ops::Index;

use std::rc::Rc;
use std::sync::Arc;

pub mod serde;

mod json;
mod yaml;

// the base64 type for serde, used by RuntimeValue
use base64_serde::base64_serde_type;
base64_serde_type!(
    RuntimeValueBase64,
    base64::engine::general_purpose::STANDARD
);

#[derive(Debug, Clone)]
pub enum RationaleResult {
    // No result.
    None,
    // There was a result.
    Same(Rc<RuntimeValue>),
    // Result was transformed.
    Transform(Rc<RuntimeValue>),
}

impl Display for RationaleResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RationaleResult::None => {
                write!(f, "None")
            }
            RationaleResult::Same(_) => {
                write!(f, "Same")
            }
            RationaleResult::Transform(_) => {
                write!(f, "Transform")
            }
        }
    }
}

impl RationaleResult {
    /// Check if there is a result.
    pub fn is_some(&self) -> bool {
        !matches!(self, RationaleResult::None)
    }

    /// Check if there is no result.
    pub fn is_none(&self) -> bool {
        !self.is_some()
    }
}

impl PartialEq<Self> for RuntimeValue {
    fn eq(&self, other: &Self) -> bool {
        match (&self, &other) {
            (Self::Boolean(lhs), Self::Boolean(rhs)) => lhs == rhs,
            (Self::Integer(lhs), Self::Integer(rhs)) => lhs == rhs,
            (Self::Decimal(lhs), Self::Decimal(rhs)) => lhs == rhs,
            (Self::String(lhs), Self::String(rhs)) => lhs == rhs,
            (Self::List(lhs), Self::List(rhs)) => lhs == rhs,
            (Self::Octets(lhs), Self::Octets(rhs)) => lhs == rhs,
            (Self::Object(lhs), Self::Object(rhs)) => lhs == rhs,
            (Self::Null, Self::Null) => true,
            // more specialness
            (Self::Octets(lhs), Self::String(rhs)) => lhs == rhs.as_bytes(),
            (Self::String(lhs), Self::Octets(rhs)) => lhs.as_bytes() == rhs,
            _ => false,
        }
    }
}

impl PartialOrd for RuntimeValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (&self, &other) {
            (Self::Boolean(lhs), Self::Boolean(rhs)) => lhs.partial_cmp(rhs),
            (Self::Integer(lhs), Self::Integer(rhs)) => lhs.partial_cmp(rhs),
            (Self::Decimal(lhs), Self::Decimal(rhs)) => lhs.partial_cmp(rhs),
            (Self::Decimal(lhs), Self::Integer(rhs)) => lhs.partial_cmp(&(*rhs as f64)),
            (Self::Integer(lhs), Self::Decimal(rhs)) => (*lhs as f64).partial_cmp(rhs),
            (Self::String(lhs), Self::String(rhs)) => lhs.partial_cmp(rhs),
            (Self::List(lhs), Self::List(rhs)) => lhs.partial_cmp(rhs),
            (Self::Octets(lhs), Self::Octets(rhs)) => lhs.partial_cmp(rhs),
            (Self::Null, Self::Null) => Some(Ordering::Equal),
            _ => None,
        }
    }
}

impl From<u8> for RuntimeValue {
    fn from(inner: u8) -> Self {
        Self::Integer(inner as _)
    }
}

impl From<u16> for RuntimeValue {
    fn from(inner: u16) -> Self {
        Self::Integer(inner as _)
    }
}

impl From<u32> for RuntimeValue {
    fn from(inner: u32) -> Self {
        Self::Integer(inner as _)
    }
}

impl From<u64> for RuntimeValue {
    fn from(inner: u64) -> Self {
        Self::Integer(inner as _)
    }
}

impl From<i16> for RuntimeValue {
    fn from(inner: i16) -> Self {
        Self::Integer(inner as _)
    }
}

impl From<i32> for RuntimeValue {
    fn from(inner: i32) -> Self {
        Self::Integer(inner as _)
    }
}

impl From<i64> for RuntimeValue {
    fn from(inner: i64) -> Self {
        Self::Integer(inner)
    }
}

impl From<usize> for RuntimeValue {
    fn from(inner: usize) -> Self {
        Self::Integer(inner as _)
    }
}

impl From<f64> for RuntimeValue {
    fn from(inner: f64) -> Self {
        Self::Decimal(inner)
    }
}

impl From<bool> for RuntimeValue {
    fn from(inner: bool) -> Self {
        Self::Boolean(inner)
    }
}

impl From<&str> for RuntimeValue {
    fn from(inner: &str) -> Self {
        Self::String(inner.to_string())
    }
}

impl<'a> From<Cow<'a, str>> for RuntimeValue {
    fn from(inner: Cow<'a, str>) -> Self {
        Self::String(inner.to_string())
    }
}

impl From<String> for RuntimeValue {
    fn from(inner: String) -> Self {
        Self::String(inner)
    }
}

impl From<Vec<u8>> for RuntimeValue {
    fn from(inner: Vec<u8>) -> Self {
        Self::Octets(inner)
    }
}

impl From<&[u8]> for RuntimeValue {
    fn from(inner: &[u8]) -> Self {
        inner.to_vec().into()
    }
}

impl<const N: usize> From<&[u8; N]> for RuntimeValue {
    fn from(inner: &[u8; N]) -> Self {
        inner.to_vec().into()
    }
}

impl From<Vec<RuntimeValue>> for RuntimeValue {
    fn from(inner: Vec<RuntimeValue>) -> Self {
        Self::List(inner.into_iter().map(Arc::new).collect())
    }
}

impl From<Vec<Arc<RuntimeValue>>> for RuntimeValue {
    fn from(value: Vec<Arc<RuntimeValue>>) -> Self {
        Self::List(value)
    }
}

impl From<&[RuntimeValue]> for RuntimeValue {
    fn from(inner: &[RuntimeValue]) -> Self {
        Self::List(inner.iter().map(|e| Arc::new(e.clone())).collect())
    }
}

impl From<Object> for RuntimeValue {
    fn from(inner: Object) -> Self {
        Self::Object(inner)
    }
}

impl<T> From<Option<T>> for RuntimeValue
where
    T: Into<RuntimeValue>,
{
    fn from(value: Option<T>) -> Self {
        if let Some(value) = value {
            value.into()
        } else {
            RuntimeValue::Null
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeValue {
    Null,
    String(String),
    Integer(i64),
    Decimal(f64),
    Boolean(bool),
    Object(Object),
    List(Vec<Arc<RuntimeValue>>),
    Octets(#[serde(with = "RuntimeValueBase64")] Vec<u8>),
}

impl Display for RuntimeValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "<null>"),
            Self::String(val) => write!(f, "{val}"),
            Self::Integer(val) => write!(f, "{val}"),
            Self::Decimal(val) => write!(f, "{val}"),
            Self::Boolean(val) => write!(f, "{val}"),
            Self::Object(val) => Display::fmt(val, f),
            Self::List(_val) => write!(f, "[ <<things>> ]"),
            Self::Octets(_val) => write!(f, "[ <<octets>> ]"),
        }
    }
}

impl RuntimeValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::String(_) => "string",
            Self::Integer(_) => "integer",
            Self::Decimal(_) => "decimal",
            Self::Boolean(_) => "boolean",
            Self::Object(_) => "object",
            Self::List(_) => "list",
            Self::Octets(_) => "octets",
        }
    }

    pub fn as_json(&self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::String(val) => serde_json::Value::String(val.clone()),
            Self::Integer(val) => serde_json::Value::Number(Number::from(*val)),
            Self::Decimal(val) => json!(val),
            Self::Boolean(val) => serde_json::Value::Bool(*val),
            Self::Object(val) => val.as_json(),
            Self::List(val) => {
                let mut inner = Vec::new();
                for each in val {
                    inner.push((**each).borrow().as_json())
                }
                serde_json::Value::Array(inner)
            }
            Self::Octets(val) => {
                let mut octets = String::new();
                for chunk in val.chunks(16) {
                    for octet in chunk {
                        octets.push_str(format!("{octet:02x} ").as_str());
                    }
                    //octets.push( '\n');
                }
                serde_json::Value::String(octets)
            }
        }
    }

    pub fn with_iter<I, T>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<RuntimeValue>,
    {
        RuntimeValue::List(iter.into_iter().map(|e| Arc::new(e.into())).collect())
    }
}

impl RuntimeValue {
    pub fn null() -> Self {
        Self::Null
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    pub fn try_get_str(&self) -> Option<&str> {
        if let Self::String(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Integer(_))
    }

    pub fn try_get_integer(&self) -> Option<i64> {
        if let Self::Integer(inner) = self {
            Some(*inner)
        } else {
            None
        }
    }

    pub fn is_decimal(&self) -> bool {
        matches!(self, Self::Decimal(_))
    }

    pub fn try_get_decimal(&self) -> Option<f64> {
        if let Self::Decimal(inner) = self {
            Some(*inner)
        } else {
            None
        }
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Boolean(_))
    }

    pub fn try_get_boolean(&self) -> Option<bool> {
        if let Self::Boolean(inner) = self {
            Some(*inner)
        } else {
            None
        }
    }

    pub fn is_list(&self) -> bool {
        matches!(self, Self::List(_))
    }

    pub fn try_get_list(&self) -> Option<&Vec<Arc<RuntimeValue>>> {
        if let Self::List(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    pub fn try_get_object(&self) -> Option<&Object> {
        if let Self::Object(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn is_octets(&self) -> bool {
        matches!(self, Self::Octets(_))
    }

    pub fn try_get_octets(&self) -> Option<&Vec<u8>> {
        if let Self::Octets(inner) = self {
            Some(inner)
        } else {
            None
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
pub struct Object(IndexMap<String, Arc<RuntimeValue>>);

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (name, value) in &self.0 {
            writeln!(f, "{}: <<{}>>", name, value.type_name())?;
        }

        Ok(())
    }
}

impl Object {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn as_json(&self) -> serde_json::Value {
        let mut inner = Map::new();
        for (name, value) in &self.0 {
            inner.insert(name.clone(), (**value).borrow().as_json());
        }

        serde_json::Value::Object(inner)
    }

    pub fn get<N>(&self, name: N) -> Option<Arc<RuntimeValue>>
    where
        N: AsRef<str>,
    {
        self.0.get(name.as_ref()).cloned()
    }

    pub fn set<N, V>(&mut self, name: N, value: V)
    where
        N: Into<String>,
        V: Into<RuntimeValue>,
    {
        self.0.insert(name.into(), Arc::new(value.into()));
    }

    pub fn with<N, V>(mut self, name: N, value: V) -> Self
    where
        N: Into<String>,
        V: Into<RuntimeValue>,
    {
        self.set(name, value);
        self
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Arc<RuntimeValue>)> {
        self.0.iter()
    }

    pub fn has_attr<A, F>(&self, name: A, f: F) -> bool
    where
        A: AsRef<str>,
        F: FnOnce(&RuntimeValue) -> bool,
    {
        if let Some(v) = &self.get(name) {
            f(v)
        } else {
            false
        }
    }

    pub fn has_str<A, B>(&self, name: A, expected: B) -> bool
    where
        A: AsRef<str>,
        B: AsRef<str>,
    {
        self.has_attr(
            name,
            |v| matches!(v, RuntimeValue::String(actual) if actual == expected.as_ref()),
        )
    }
}

impl Index<&str> for Object {
    type Output = RuntimeValue;

    fn index(&self, index: &str) -> &Self::Output {
        const NULL: RuntimeValue = RuntimeValue::Null;
        self.0.get(index).map(|s| s.as_ref()).unwrap_or(&NULL)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::Value;

    pub(crate) fn assert_yaml<F, E>(f: F)
    where
        F: FnOnce(&str) -> Result<RuntimeValue, E>,
        E: std::error::Error,
    {
        let value: RuntimeValue = f(r#"
foo: bar
bar:
  - 1
  - "baz"
  - true
here:
  comes: ~
  1: foo

"#)
        .unwrap();
        assert_eq!(
            RuntimeValue::Object({
                let mut o = Object::new();
                o.set(
                    "bar",
                    vec![
                        Arc::new(1i64.into()),
                        Arc::new("baz".into()),
                        Arc::new(true.into()),
                    ],
                );
                o.set("foo", "bar");
                o.set("here", {
                    let mut o = Object::new();
                    o.set("comes", RuntimeValue::Null);
                    o.set("1", "foo");
                    o
                });
                o
            }),
            value
        );
    }

    #[test]
    fn test_obj_has_str() {
        let mut o = Object::new();
        o.set("foo", "bar");
        o.set("bar", 1);
        o.set("null", RuntimeValue::Null);

        assert!(o.has_str("foo", "bar"));
        assert!(!o.has_str("bar", "1"));
        assert!(!o.has_str("baz", ""));
        assert!(!o.has_str("null", ""));
    }

    #[test]
    fn test_serde_rv_boolean() {
        assert_eq_and_back_again(RuntimeValue::Boolean(true), json!({"boolean": true}));
        assert_eq_and_back_again(RuntimeValue::Boolean(false), json!({"boolean": false}));
    }

    #[test]
    fn test_serde_rv_integer() {
        assert_eq_and_back_again(RuntimeValue::Integer(42), json!({"integer": 42}));
        assert_eq_and_back_again(RuntimeValue::Integer(0), json!({"integer": 0}));
        assert_eq_and_back_again(RuntimeValue::Integer(-42), json!({"integer": -42}));
    }

    #[test]
    fn test_serde_rv_decimal() {
        assert_eq_and_back_again(RuntimeValue::Decimal(2.3), json!({"decimal": 2.3}));
        assert_eq_and_back_again(RuntimeValue::Decimal(0.0), json!({"decimal": 0.0}));
        assert_eq_and_back_again(RuntimeValue::Decimal(-2.3), json!({"decimal": -2.3}));
    }

    #[test]
    fn test_serde_rv_string() {
        assert_eq_and_back_again(
            RuntimeValue::String("2.3".to_string()),
            json!({"string": "2.3"}),
        );
        assert_eq_and_back_again(
            RuntimeValue::String("null".to_string()),
            json!({"string": "null"}),
        );
        assert_eq_and_back_again(
            RuntimeValue::String("-norway".to_string()),
            json!({"string": "-norway"}),
        );
    }

    #[test]
    fn test_serde_rv_null() {
        assert_eq_and_back_again(RuntimeValue::Null, json!("null"));
    }

    #[test]
    fn test_serde_rv_octets() {
        assert_eq_and_back_again(
            RuntimeValue::Octets(b"Foo Bar".to_vec()),
            json!({"octets": "Rm9vIEJhcg=="}),
        );
    }

    #[test]
    fn test_serde_rv_list() {
        assert_eq_and_back_again(
            RuntimeValue::from(vec![
                RuntimeValue::Null,
                RuntimeValue::String("1.2".to_string()),
                RuntimeValue::Boolean(true),
                RuntimeValue::Integer(42),
                RuntimeValue::from(vec![
                    RuntimeValue::Null,
                    RuntimeValue::String("1.2".to_string()),
                    RuntimeValue::Boolean(true),
                    RuntimeValue::Integer(42),
                ]),
            ]),
            json!({
                "list": [
                    "null",
                    {"string": "1.2"},
                    {"boolean": true},
                    {"integer":  42},
                    { "list": [
                        "null",
                        {"string": "1.2"},
                        {"boolean": true},
                        {"integer": 42}]
                    }
                ]
            }),
        );
    }

    #[test]
    fn test_serde_rv_object() {
        assert_eq_and_back_again(
            RuntimeValue::from(
                Object::new()
                    .with("f_null", RuntimeValue::Null)
                    .with("f_integer", 42)
                    .with("f_boolean", false)
                    .with("f_string", "foo")
                    .with("f_list", vec![RuntimeValue::from(42)])
                    .with(
                        "f_object",
                        Object::new()
                            .with("f_boolean", false)
                            .with("f_list", vec![RuntimeValue::Decimal(2.3f64)]),
                    )
                    .with("f_emptyObject", Object::new())
                    .with("f_emptyList", RuntimeValue::List(vec![])),
            ),
            json!({
                "object": {
                    "f_boolean": { "boolean": false },
                    "f_emptyList": { "list": [] },
                    "f_emptyObject": { "object": {} },
                    "f_integer": { "integer": 42 },
                    "f_list": { "list": [ { "integer": 42 } ] },
                    "f_null": "null",
                    "f_object": {
                        "object": {
                            "f_boolean": { "boolean": false },
                            "f_list": { "list": [ { "decimal": 2.3f64 } ] },
                        }
                    },
                    "f_string": { "string": "foo" }
                }
            }),
        );
    }

    /// test that serializing an deserializing a value yields the same result
    fn assert_eq_and_back_again(value: RuntimeValue, expected_json: Value) {
        let json = serde_json::to_value(&value).unwrap();
        assert_eq!(expected_json, json);
        let json = serde_json::to_string(&json).unwrap();
        let deserialized = serde_json::from_str(&json).unwrap();
        assert_eq!(value, deserialized);
    }
}
