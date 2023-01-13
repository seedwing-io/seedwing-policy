use crate::core::Function;
use crate::lang::lir;
use crate::lang::lir::{Field, InnerType};
use crate::lang::mir::TypeHandle;
use crate::lang::parser::expr::Expr;
use crate::lang::parser::Located;
use crate::runtime::RuntimeError;
use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{json, Map, Number};
use std::any::Any;
use std::borrow::Borrow;
use std::cell::{Ref, RefCell};
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter, Pointer};
use std::future::{ready, Future};
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

mod json;

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
pub enum Rationale {
    //TypeHandle(Arc<>),
    Type(Arc<lir::Type>),
    Field(Arc<lir::Field>),
    Expr(Arc<Located<Expr>>),
}

impl Rationale {
    pub fn id(&self) -> u64 {
        match self {
            Rationale::Type(t) => t.id,
            Rationale::Field(f) => f.id,
            Rationale::Expr(e) => e.id,
        }
    }

    pub fn description(&self) -> Option<String> {
        match self {
            Rationale::Type(t) => t.name().map(|inner| inner.as_type_str()),
            Rationale::Field(f) => Some(f.name()),
            Rationale::Expr(_) => Some("expression".into()),
        }
    }
}

impl Hash for Rationale {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state)
    }
}

impl PartialEq<Self> for Rationale {
    fn eq(&self, other: &Self) -> bool {
        self.id().eq(&other.id())
    }
}

impl Eq for Rationale {}

impl From<Arc<lir::Type>> for Rationale {
    fn from(inner: Arc<lir::Type>) -> Self {
        Self::Type(inner)
    }
}

impl From<Arc<lir::Field>> for Rationale {
    fn from(inner: Arc<lir::Field>) -> Self {
        Self::Field(inner)
    }
}

impl From<Arc<Located<Expr>>> for Rationale {
    fn from(inner: Arc<Located<Expr>>) -> Self {
        Self::Expr(inner)
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
    pub fn is_some(&self) -> bool {
        !matches!(self, RationaleResult::None)
    }

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
            _ => None,
        }
    }
}

impl From<&str> for RuntimeValue {
    fn from(inner: &str) -> Self {
        Self::String(inner.to_string())
    }
}

impl From<u8> for RuntimeValue {
    fn from(inner: u8) -> Self {
        Self::Integer(inner as _)
    }
}

impl From<u32> for RuntimeValue {
    fn from(inner: u32) -> Self {
        Self::Integer(inner as _)
    }
}

impl From<i64> for RuntimeValue {
    fn from(inner: i64) -> Self {
        Self::Integer(inner)
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
        Self::List(inner.iter().map(|e| Rc::new(e.clone())).collect())
    }
}

impl From<Object> for RuntimeValue {
    fn from(inner: Object) -> Self {
        Self::Object(inner)
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
    List(#[serde(skip)] Vec<Rc<RuntimeValue>>),
    Octets(Vec<u8>),
}

impl Display for RuntimeValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "<null>"),
            Self::String(val) => write!(f, "{}", val),
            Self::Integer(val) => write!(f, "{}", val),
            Self::Decimal(val) => write!(f, "{}", val),
            Self::Boolean(val) => write!(f, "{}", val),
            Self::Object(val) => Display::fmt(val, f),
            Self::List(val) => write!(f, "[ <<things>> ]"),
            Self::Octets(val) => write!(f, "[ <<octets>> ]"),
        }
    }
}

impl RuntimeValue {
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
                        octets.push_str(format!("{:02x} ", octet).as_str());
                    }
                    //octets.push( '\n');
                }
                serde_json::Value::String(octets)
            }
        }
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

    pub fn try_get_list(&self) -> Option<&Vec<Rc<RuntimeValue>>> {
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

#[derive(Serialize, Debug, Clone, Default)]
pub struct Object {
    #[serde(skip)]
    fields: IndexMap<String, Rc<RuntimeValue>>,
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (name, value) in &self.fields {
            writeln!(f, "{}: <<value>>", name)?;
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

    fn as_json(&self) -> serde_json::Value {
        let mut inner = Map::new();
        for (name, value) in &self.fields {
            inner.insert(name.clone(), (**value).borrow().as_json());
        }

        serde_json::Value::Object(inner)
    }

    pub fn get(&self, name: String) -> Option<Rc<RuntimeValue>> {
        self.fields.get(&name).cloned()
    }

    pub fn set(&mut self, name: String, value: RuntimeValue) {
        self.fields.insert(name, Rc::new(value));
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Rc<RuntimeValue>)> {
        self.fields.iter()
    }
}
