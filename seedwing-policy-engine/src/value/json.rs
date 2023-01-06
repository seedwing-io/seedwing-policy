use crate::value::{InnerValue, InputValue, Object};
use serde_json::{Number, Value as JsonValue};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

impl From<&JsonValue> for InputValue {
    fn from(value: &JsonValue) -> Self {
        let inner = InnerValue::from(value);
        inner.into()
    }
}

impl From<JsonValue> for InputValue {
    fn from(value: JsonValue) -> Self {
        let inner = InnerValue::from(value);
        inner.into()
    }
}

impl<T: Borrow<JsonValue>> From<T> for InnerValue {
    fn from(value: T) -> Self {
        match value.borrow() {
            JsonValue::Null => InnerValue::Null,
            JsonValue::Bool(inner) => InnerValue::Boolean(*inner),
            JsonValue::Number(inner) => {
                if inner.is_f64() {
                    InnerValue::Decimal(inner.as_f64().unwrap())
                } else if inner.is_i64() {
                    InnerValue::Integer(inner.as_i64().unwrap())
                } else {
                    todo!("u64 is needed, I guess")
                }
            }
            JsonValue::String(inner) => InnerValue::String(inner.clone()),
            JsonValue::Array(inner) => {
                InnerValue::List(inner.iter().map(|e| Rc::new(InputValue::from(e))).collect())
            }
            JsonValue::Object(inner) => {
                let fields = inner
                    .iter()
                    .map(|(k, v)| (k.clone(), Rc::new(InputValue::from(v))))
                    .collect();

                InnerValue::Object(Object { fields })
            }
        }
    }
}
