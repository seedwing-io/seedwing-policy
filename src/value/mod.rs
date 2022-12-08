use std::collections::HashMap;
use std::rc::Rc;
use crate::lang::Located;
use crate::runtime::{RuntimeError, RuntimeType};

mod json;

#[derive(Debug)]
pub struct Value {
    inner: InnerValue,
    matches: Vec<Rc<Located<RuntimeType>>>,
    nonmatches: Vec<Rc<Located<RuntimeType>>>,
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

#[derive(Debug)]
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
    pub(crate) fn note(&mut self, ty: Rc<Located<RuntimeType>>, matches: bool) {
        if matches {
            self.matches.push(ty);
        } else {
            self.nonmatches.push(ty);
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

#[derive(Debug)]
pub struct Object {
    fields: HashMap<String, Value>
}

impl Object {

    pub fn get(&mut self, name: String) -> Option<&mut Value> {
        self.fields.get_mut(&name)

    }

}