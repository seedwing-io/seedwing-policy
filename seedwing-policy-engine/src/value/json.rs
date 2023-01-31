use crate::value::{Object, RuntimeValue};
use serde_json::{Number, Value as JsonValue};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/*
impl From<&JsonValue> for RuntimeValue {
    fn from(value: &JsonValue) -> Self {
        let inner = RuntimeValue::from(value);
        inner.into()
    }
}

impl From<JsonValue> for RuntimeValue {
    fn from(value: JsonValue) -> Self {
        let inner = InnerValue::from(value);
        inner.into()
    }
}
 */

impl From<JsonValue> for RuntimeValue {
    fn from(value: JsonValue) -> Self {
        match value {
            JsonValue::Null => RuntimeValue::Null,
            JsonValue::Bool(inner) => RuntimeValue::Boolean(inner),
            JsonValue::Number(inner) => {
                if inner.is_f64() {
                    RuntimeValue::Decimal(inner.as_f64().unwrap())
                } else if inner.is_i64() {
                    RuntimeValue::Integer(inner.as_i64().unwrap())
                } else {
                    todo!("u64 is needed, I guess")
                }
            }
            JsonValue::String(inner) => RuntimeValue::String(inner),
            JsonValue::Array(inner) => RuntimeValue::List(
                inner
                    .into_iter()
                    .map(|e| Rc::new(RuntimeValue::from(e)))
                    .collect(),
            ),
            JsonValue::Object(inner) => {
                let fields = inner
                    .into_iter()
                    .map(|(k, v)| (k, Rc::new(RuntimeValue::from(v))))
                    .collect();

                RuntimeValue::Object(Object { fields })
            }
        }
    }
}

impl From<&JsonValue> for RuntimeValue {
    fn from(value: &JsonValue) -> Self {
        match value {
            JsonValue::Null => RuntimeValue::Null,
            JsonValue::Bool(inner) => RuntimeValue::Boolean(*inner),
            JsonValue::Number(inner) => {
                if inner.is_f64() {
                    RuntimeValue::Decimal(inner.as_f64().unwrap())
                } else if inner.is_i64() {
                    RuntimeValue::Integer(inner.as_i64().unwrap())
                } else {
                    todo!("u64 is needed, I guess")
                }
            }
            JsonValue::String(inner) => RuntimeValue::String(inner.clone()),
            JsonValue::Array(inner) => RuntimeValue::List(
                inner
                    .iter()
                    .map(|e| Rc::new(RuntimeValue::from(e)))
                    .collect(),
            ),
            JsonValue::Object(inner) => {
                let fields = inner
                    .iter()
                    .map(|(k, v)| (k.clone(), Rc::new(RuntimeValue::from(v))))
                    .collect();

                RuntimeValue::Object(Object { fields })
            }
        }
    }
}
