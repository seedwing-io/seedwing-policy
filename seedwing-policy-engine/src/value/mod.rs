use crate::core::Function;
use crate::lang::lir::{Field, Type};
use crate::lang::parser::expr::Expr;
use crate::lang::parser::Located;
use crate::lang::TypeName;
use crate::runtime::{RuntimeError, TypeHandle};
use async_mutex::Mutex;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::future::{ready, Future};
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
pub enum Noted {
    TypeHandle(Arc<TypeHandle>),
    Type(Arc<Located<Type>>),
    Field(Arc<Located<Field>>),
    Expr(Arc<Located<Expr>>),
}

impl From<Arc<TypeHandle>> for Noted {
    fn from(inner: Arc<TypeHandle>) -> Self {
        Self::TypeHandle(inner)
    }
}

impl From<Arc<Located<Type>>> for Noted {
    fn from(inner: Arc<Located<Type>>) -> Self {
        Self::Type(inner)
    }
}

impl From<Arc<Located<Field>>> for Noted {
    fn from(inner: Arc<Located<Field>>) -> Self {
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

impl From<&str> for Value {
    fn from(inner: &str) -> Self {
        InnerValue::String(inner.to_string()).into()
    }
}

impl From<u8> for Value {
    fn from(inner: u8) -> Self {
        InnerValue::Integer(inner as _).into()
    }
}

impl From<u32> for Value {
    fn from(inner: u32) -> Self {
        InnerValue::Integer(inner as _).into()
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

impl From<&[u8]> for Value {
    fn from(inner: &[u8]) -> Self {
        inner.to_vec().into()
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

impl From<Object> for Value {
    fn from(inner: Object) -> Self {
        InnerValue::Object(inner).into()
    }
}

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

impl InnerValue {
    fn display<'p>(&'p self, printer: &'p mut Printer) -> Pin<Box<dyn Future<Output = ()> + 'p>> {
        match self {
            InnerValue::Null => Box::pin(async move {
                printer.write("<null>");
            }),
            InnerValue::String(inner) => {
                Box::pin(async move { printer.write(format!("\"{}\"", inner).as_str()) })
            }
            InnerValue::Integer(inner) => {
                Box::pin(async move { printer.write(format!("{}", inner).as_str()) })
            }
            InnerValue::Decimal(inner) => {
                Box::pin(async move { printer.write(format!("{}", inner).as_str()) })
            }
            InnerValue::Boolean(inner) => {
                Box::pin(async move { printer.write(format!("{}", inner).as_str()) })
            }
            InnerValue::Object(inner) => Box::pin(async move { inner.display(printer).await }),
            InnerValue::List(inner) => Box::pin(async move {
                printer.write("[ ");
                for item in inner {
                    item.lock().await.inner.display(printer).await;
                    printer.write(", ");
                }
                printer.write(" ]");
            }),
            InnerValue::Octets(inner) => {
                Box::pin(async move {
                    // todo write in columns of bytes like a byte inspector
                    for byte in inner {
                        printer.write(format!("{:0x}", byte).as_str())
                    }
                })
            }
        }
    }
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
        matches!(self.inner, InnerValue::String(_))
    }

    pub fn try_get_string(&self) -> Option<String> {
        if let InnerValue::String(inner) = &self.inner {
            Some(inner.clone())
        } else {
            None
        }
    }

    pub fn is_integer(&self) -> bool {
        matches!(self.inner, InnerValue::Integer(_))
    }

    pub fn try_get_integer(&self) -> Option<i64> {
        if let InnerValue::Integer(inner) = &self.inner {
            Some(*inner)
        } else {
            None
        }
    }

    pub fn is_decimal(&self) -> bool {
        matches!(self.inner, InnerValue::Decimal(_))
    }

    pub fn try_get_decimal(&self) -> Option<f64> {
        if let InnerValue::Decimal(inner) = &self.inner {
            Some(*inner)
        } else {
            None
        }
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self.inner, InnerValue::Boolean(_))
    }

    pub fn try_get_boolean(&self) -> Option<bool> {
        if let InnerValue::Boolean(inner) = &self.inner {
            Some(*inner)
        } else {
            None
        }
    }

    pub fn is_list(&self) -> bool {
        matches!(self.inner, InnerValue::List(_))
    }

    pub fn try_get_list(&self) -> Option<&Vec<Arc<Mutex<Value>>>> {
        if let InnerValue::List(inner) = &self.inner {
            Some(inner)
        } else {
            None
        }
    }

    pub fn is_object(&self) -> bool {
        matches!(self.inner, InnerValue::Object(_))
    }

    pub fn try_get_object(&self) -> Option<&Object> {
        if let InnerValue::Object(inner) = &self.inner {
            Some(inner)
        } else {
            None
        }
    }

    pub fn is_octets(&self) -> bool {
        matches!(self.inner, InnerValue::Octets(_))
    }

    pub fn try_get_octets(&self) -> Option<&Vec<u8>> {
        if let InnerValue::Octets(inner) = &self.inner {
            Some(inner)
        } else {
            None
        }
    }

    pub async fn display(&self) -> String {
        let mut printer = Printer::new();
        self.inner.display(&mut printer).await;
        printer.content
    }
}

#[derive(Debug, Clone, Default)]
pub struct Object {
    fields: HashMap<String, Arc<Mutex<Value>>>,
}

impl Object {
    pub fn new() -> Self {
        Self {
            fields: Default::default(),
        }
    }

    async fn display(&self, printer: &mut Printer) {
        printer.write("{");
        printer.indent += 1;
        for (name, value) in &self.fields {
            printer.write_with_indent(name.as_str());
            printer.write(": ");
            value.lock().await.inner.display(printer).await;
            printer.write(",");
        }
        printer.indent -= 1;
        printer.write_with_indent("}");
    }

    pub fn get(&self, name: String) -> Option<Arc<Mutex<Value>>> {
        self.fields.get(&name).cloned()
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.fields.insert(name, Arc::new(Mutex::new(value)));
    }
}
