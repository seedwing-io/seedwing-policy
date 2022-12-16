use crate::function::Function;
use crate::lang::expr::Expr;
use crate::lang::ty::TypeName;
use crate::lang::Located;
use crate::runtime::{RuntimeError, RuntimeField, RuntimeType, TypeHandle};
use async_mutex::Mutex;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

mod json;

#[derive(Debug, Clone)]
pub enum Noted {
    TypeHandle(Arc<TypeHandle>),
    Type(Arc<Located<RuntimeType>>),
    Field(Arc<Located<RuntimeField>>),
    Expr(Arc<Located<Expr>>),
}

impl From<Arc<TypeHandle>> for Noted {
    fn from(inner: Arc<TypeHandle>) -> Self {
        Self::TypeHandle(inner)
    }
}

impl From<Arc<Located<RuntimeType>>> for Noted {
    fn from(inner: Arc<Located<RuntimeType>>) -> Self {
        Self::Type(inner)
    }
}

impl From<Arc<Located<RuntimeField>>> for Noted {
    fn from(inner: Arc<Located<RuntimeField>>) -> Self {
        Self::Field(inner)
    }
}

impl From<Arc<Located<Expr>>> for Noted {
    fn from(inner: Arc<Located<Expr>>) -> Self {
        Self::Expr(inner)
    }
}

#[derive(Debug, Clone)]
pub struct Value {
    inner: InnerValue,
    matches: Vec<Noted>,
    nonmatches: Vec<Noted>,
    transforms: HashMap<TypeName, Arc<Mutex<Value>>>,
}

impl PartialEq<Self> for Value {
    fn eq(&self, other: &Self) -> bool {
        match (&self.inner, &other.inner) {
            (InnerValue::Boolean(lhs), InnerValue::Boolean(rhs)) => lhs == rhs,
            (InnerValue::Integer(lhs), InnerValue::Integer(rhs)) => lhs == rhs,
            (InnerValue::Decimal(lhs), InnerValue::Decimal(rhs)) => lhs == rhs,
            (InnerValue::String(lhs), InnerValue::String(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (&self.inner, &other.inner) {
            (InnerValue::Boolean(lhs), InnerValue::Boolean(rhs)) => lhs.partial_cmp(rhs),
            (InnerValue::Integer(lhs), InnerValue::Integer(rhs)) => lhs.partial_cmp(rhs),
            (InnerValue::Decimal(lhs), InnerValue::Decimal(rhs)) => lhs.partial_cmp(rhs),
            (InnerValue::Decimal(lhs), InnerValue::Integer(rhs)) => lhs.partial_cmp(&(*rhs as f64)),
            (InnerValue::Integer(lhs), InnerValue::Decimal(rhs)) => (*lhs as f64).partial_cmp(rhs),
            (InnerValue::String(lhs), InnerValue::String(rhs)) => lhs.partial_cmp(rhs),
            _ => None,
        }
    }
}

impl From<i64> for Value {
    fn from(inner: i64) -> Self {
        InnerValue::Integer(inner).into()
    }
}

impl From<f64> for Value {
    fn from(inner: f64) -> Self {
        InnerValue::Decimal(inner).into()
    }
}

impl From<bool> for Value {
    fn from(inner: bool) -> Self {
        InnerValue::Boolean(inner).into()
    }
}

impl From<String> for Value {
    fn from(inner: String) -> Self {
        InnerValue::String(inner).into()
    }
}

impl From<Vec<u8>> for Value {
    fn from(inner: Vec<u8>) -> Self {
        InnerValue::Octets(inner).into()
    }
}

impl From<Vec<Value>> for Value {
    fn from(inner: Vec<Value>) -> Self {
        InnerValue::List(
            inner
                .iter()
                .map(|e| Arc::new(Mutex::new(e.clone())))
                .collect(),
        )
        .into()
    }
}

impl Value {}

impl From<InnerValue> for Value {
    fn from(inner: InnerValue) -> Self {
        Self {
            inner,
            matches: vec![],
            nonmatches: vec![],
            transforms: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum InnerValue {
    Null,
    String(String),
    Integer(i64),
    Decimal(f64),
    Boolean(bool),
    Object(Object),
    List(Vec<Arc<Mutex<Value>>>),
    Octets(Vec<u8>),
}

impl Value {
    pub(crate) fn note<N: Into<Noted>>(&mut self, noted: N, matches: bool) {
        if matches {
            self.matches.push(noted.into());
        } else {
            self.nonmatches.push(noted.into());
        }
    }

    pub(crate) fn transform(&mut self, name: TypeName, value: Arc<Mutex<Value>>) {
        self.transforms.insert(name, value);
    }

    pub fn is_string(&self) -> bool {
        match &self.inner {
            InnerValue::String(_) => true,
            _ => false,
        }
    }

    pub fn try_get_string(&self) -> Option<String> {
        if let InnerValue::String(inner) = &self.inner {
            Some(inner.clone())
        } else {
            None
        }
    }

    pub fn is_integer(&self) -> bool {
        match &self.inner {
            InnerValue::Integer(_) => true,
            _ => false,
        }
    }

    pub fn try_get_integer(&self) -> Option<i64> {
        if let InnerValue::Integer(inner) = &self.inner {
            Some(*inner)
        } else {
            None
        }
    }

    pub fn is_decimal(&self) -> bool {
        match &self.inner {
            InnerValue::Decimal(_) => true,
            _ => false,
        }
    }

    pub fn try_get_decimal(&self) -> Option<f64> {
        if let InnerValue::Decimal(inner) = &self.inner {
            Some(*inner)
        } else {
            None
        }
    }

    pub fn is_boolean(&self) -> bool {
        match &self.inner {
            InnerValue::Boolean(_) => true,
            _ => false,
        }
    }

    pub fn try_get_boolean(&self) -> Option<bool> {
        if let InnerValue::Boolean(inner) = &self.inner {
            Some(*inner)
        } else {
            None
        }
    }

    pub fn is_list(&self) -> bool {
        match &self.inner {
            InnerValue::List(_) => true,
            _ => false,
        }
    }

    pub fn try_get_list(&self) -> Option<&Vec<Arc<Mutex<Value>>>> {
        if let InnerValue::List(inner) = &self.inner {
            Some(inner)
        } else {
            None
        }
    }

    pub fn is_object(&self) -> bool {
        match &self.inner {
            InnerValue::Object(_) => true,
            _ => false,
        }
    }

    pub fn try_get_object(&self) -> Option<&Object> {
        if let InnerValue::Object(inner) = &self.inner {
            Some(inner)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Object {
    fields: HashMap<String, Arc<Mutex<Value>>>,
}

impl Object {
    pub fn get(&self, name: String) -> Option<Arc<Mutex<Value>>> {
        self.fields.get(&name).cloned()
    }
}
