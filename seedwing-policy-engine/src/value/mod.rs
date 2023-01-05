use crate::core::Function;
use crate::lang::lir::{Field, InnerType};
use crate::lang::mir::TypeHandle;
use crate::lang::parser::expr::Expr;
use crate::lang::parser::Located;
use crate::lang::{lir, TypeName};
use crate::runtime::RuntimeError;
use async_mutex::Mutex;
use serde::Serialize;
use serde_json::{json, Map, Number};
use std::any::Any;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
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
    Same(Arc<Mutex<Value>>),
    Transform(Arc<Mutex<Value>>),
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

#[derive(Serialize, Debug, Clone)]
pub struct Value {
    #[serde(flatten)]
    inner: InnerValue,
    #[serde(skip)]
    rational: Vec<(Rationale, RationaleResult)>,
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
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
            rational: Default::default(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub enum InnerValue {
    Null,
    String(String),
    Integer(i64),
    Decimal(f64),
    Boolean(bool),
    Object(Object),
    List(#[serde(skip)] Vec<Arc<Mutex<Value>>>),
    Octets(Vec<u8>),
}

impl Display for InnerValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InnerValue::Null => write!(f, "<null>"),
            InnerValue::String(val) => write!(f, "{}", val),
            InnerValue::Integer(val) => write!(f, "{}", val),
            InnerValue::Decimal(val) => write!(f, "{}", val),
            InnerValue::Boolean(val) => write!(f, "{}", val),
            InnerValue::Object(val) => Display::fmt(val, f),
            InnerValue::List(val) => write!(f, "[ <<things>> ]"),
            InnerValue::Octets(val) => write!(f, "[ <<octets>> ]"),
        }
    }
}

impl InnerValue {
    fn as_json<'p>(&'p self) -> Pin<Box<dyn Future<Output = serde_json::Value> + 'p>> {
        match self {
            InnerValue::Null => Box::pin(async move { serde_json::Value::Null }),
            InnerValue::String(val) => {
                Box::pin(async move { serde_json::Value::String(val.clone()) })
            }
            InnerValue::Integer(val) => {
                Box::pin(async move { serde_json::Value::Number(Number::from(*val)) })
            }
            InnerValue::Decimal(val) => Box::pin(async move { json!(val) }),
            InnerValue::Boolean(val) => Box::pin(async move { serde_json::Value::Bool(*val) }),
            InnerValue::Object(val) => Box::pin(async move { val.as_json().await }),
            InnerValue::List(val) => Box::pin(async move {
                let mut inner = Vec::new();
                for each in val {
                    inner.push(each.lock().await.as_json().await)
                }
                serde_json::Value::Array(inner)
            }),
            InnerValue::Octets(val) => {
                Box::pin(async move { serde_json::Value::String("octets".into()) })
            }
        }
    }
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
    pub(crate) fn rationale<N: Into<Rationale>>(
        &mut self,
        rationale: N,
        result: RationaleResult,
    ) -> RationaleResult {
        self.rational.push((rationale.into(), result.clone()));
        result
    }

    pub fn get_rationale(&self) -> &Vec<(Rationale, RationaleResult)> {
        &self.rational
    }

    pub fn inner(&self) -> &InnerValue {
        &self.inner
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

    pub async fn as_json(&self) -> serde_json::Value {
        self.inner.as_json().await
    }
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct Object {
    #[serde(skip)]
    fields: HashMap<String, Arc<Mutex<Value>>>,
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

    async fn as_json(&self) -> serde_json::Value {
        let mut inner = Map::new();
        for (name, value) in &self.fields {
            inner.insert(name.clone(), value.lock().await.as_json().await);
        }

        serde_json::Value::Object(inner)
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

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Arc<Mutex<Value>>)> {
        self.fields.iter()
    }
}
