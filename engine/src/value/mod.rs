//! Value representations in the policy engine.
//!
//! Values are inputs or outputs from patterns, and can be constructed automatically from JSON and other primitives.

use ::serde::Serialize;
use indexmap::IndexMap;
use serde_json::{json, Map, Number};

use std::borrow::{Borrow, Cow};

use std::cmp::Ordering;

use std::fmt::{Debug, Display, Formatter};

use std::hash::Hasher;
use std::ops::Index;

use std::rc::Rc;
use std::sync::Arc;

mod serde;

mod json;
mod yaml;

struct Printer {
    indent: u8,
    content: String,
}

impl Printer {
    fn new() -> Self {
        Self {
            indent: 0,
            content: String::new(),
        }
    }

    fn write(&mut self, value: &str) {
        self.content.push_str(value);
    }

    fn write_with_indent(&mut self, value: &str) {
        self.content.push('\n');
        self.content.push_str(self.indent().as_str());
        self.content.push_str(value);
    }

    fn indent(&self) -> String {
        let mut spacing = String::new();
        for _ in 0..(self.indent * 2) {
            spacing.push(' ');
        }
        spacing
    }
}

#[derive(Debug, Clone)]
pub enum RationaleResult {
    None,
    Same(Rc<RuntimeValue>),
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

#[derive(Serialize, Debug, Clone)]
pub enum RuntimeValue {
    Null,
    String(String),
    Integer(i64),
    Decimal(f64),
    Boolean(bool),
    Object(Object),
    List(#[serde(skip)] Vec<Arc<RuntimeValue>>),
    Octets(Vec<u8>),
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

    pub fn try_get_string(&self) -> Option<String> {
        if let Self::String(inner) = self {
            Some(inner.clone())
        } else {
            None
        }
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

#[derive(Serialize, Debug, Clone, Default, PartialEq)]
pub struct Object {
    #[serde(skip)]
    fields: IndexMap<String, Arc<RuntimeValue>>,
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (name, value) in &self.fields {
            writeln!(f, "{}: <<{}>>", name, value.type_name())?;
        }

        Ok(())
    }
}

impl Object {
    pub fn new() -> Self {
        Self {
            fields: Default::default(),
        }
    }

    pub fn as_json(&self) -> serde_json::Value {
        let mut inner = Map::new();
        for (name, value) in &self.fields {
            inner.insert(name.clone(), (**value).borrow().as_json());
        }

        serde_json::Value::Object(inner)
    }

    pub fn get<N>(&self, name: N) -> Option<Arc<RuntimeValue>>
    where
        N: AsRef<str>,
    {
        self.fields.get(name.as_ref()).cloned()
    }

    pub fn set<N, V>(&mut self, name: N, value: V)
    where
        N: Into<String>,
        V: Into<RuntimeValue>,
    {
        self.fields.insert(name.into(), Arc::new(value.into()));
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Arc<RuntimeValue>)> {
        self.fields.iter()
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
        self.fields.get(index).map(|s| s.as_ref()).unwrap_or(&NULL)
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
                o.set("bar", {
                    let mut s = Vec::new();
                    s.push(Arc::new(1i64.into()));
                    s.push(Arc::new("baz".into()));
                    s.push(Arc::new(true.into()));
                    s
                });
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
}
