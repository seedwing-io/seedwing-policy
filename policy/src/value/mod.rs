use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;
use crate::lang::expr::Expr;
use crate::lang::Located;
use crate::runtime::{RuntimeError, RuntimeField, RuntimeType};

mod json;

#[derive(Debug, Clone)]
pub enum Noted {
    Type(Rc<Located<RuntimeType>>),
    Field(Rc<Located<RuntimeField>>),
    Expr(Rc<Located<Expr>>),
}

impl From<Rc<Located<RuntimeType>>> for Noted {
    fn from(inner: Rc<Located<RuntimeType>>) -> Self {
        Self::Type(inner)
    }
}

impl From<Rc<Located<RuntimeField>>> for Noted {
    fn from(inner: Rc<Located<RuntimeField>>) -> Self {
        Self::Field(inner)
    }
}

impl From<Rc<Located<Expr>>> for Noted {
    fn from(inner: Rc<Located<Expr>>) -> Self {
        Self::Expr(inner)
    }
}

#[derive(Debug, Clone)]
pub struct Value {
    inner: InnerValue,
    matches: Vec<Noted>,
    nonmatches: Vec<Noted>,
}

impl PartialEq<Self> for Value {
    fn eq(&self, other: &Self) -> bool {
        match (&self.inner, &other.inner)  {
            (InnerValue::Boolean(lhs), InnerValue::Boolean(rhs)) => {
                lhs == rhs
            }
            (InnerValue::Integer(lhs), InnerValue::Integer(rhs)) => {
                lhs == rhs
            }
            (InnerValue::Decimal(lhs), InnerValue::Decimal(rhs)) => {
                lhs == rhs
            }
            (InnerValue::String(lhs), InnerValue::String(rhs)) => {
                lhs == rhs
            }
            _ => false
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (&self.inner, &other.inner) {
            (InnerValue::Boolean(lhs), InnerValue::Boolean(rhs)) => {
                lhs.partial_cmp(rhs)
            }
            (InnerValue::Integer(lhs), InnerValue::Integer(rhs)) => {
                lhs.partial_cmp(rhs)
            }
            (InnerValue::Decimal(lhs), InnerValue::Decimal(rhs)) => {
                lhs.partial_cmp(rhs)
            }
            (InnerValue::Decimal(lhs), InnerValue::Integer(rhs)) => {
                lhs.partial_cmp(&(*rhs as f64) )
            }
            (InnerValue::Integer(lhs), InnerValue::Decimal(rhs)) => {
                (*lhs as f64).partial_cmp(rhs)
            }
            (InnerValue::String(lhs), InnerValue::String(rhs)) => {
                lhs.partial_cmp(rhs)
            }
            _ => None
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

impl Value {

}

impl From<InnerValue> for Value {
    fn from(inner: InnerValue) -> Self {
        Self {
            inner,
            matches: vec![],
            nonmatches: vec![]
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
    List(Vec<InnerValue>),
}

impl Value {
    pub(crate) fn note<N: Into<Noted>>(&mut self, noted: N, matches: bool) {
        if matches {
            self.matches.push(noted.into());
        } else {
            self.nonmatches.push(noted.into());
        }
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

    pub fn is_object(&self) -> bool {
        match &self.inner {
            InnerValue::Object(_) => true,
            _ => false,
        }
    }

    pub fn try_get_object(&mut self) -> Option<&mut Object> {
        if let InnerValue::Object(inner) = &mut self.inner {
            Some(inner)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Object {
    fields: HashMap<String, Value>
}

impl Object {

    pub fn get(&mut self, name: String) -> Option<&mut Value> {
        self.fields.get_mut(&name)

    }

}