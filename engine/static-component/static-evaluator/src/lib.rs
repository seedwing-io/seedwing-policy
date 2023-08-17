use std::str;

wit_bindgen::generate!({
    path: "../../wit/",
    world: "static-evaluator",
    exports: {
        world: Export,
    },
});

use crate::seedwing::policy::engine;
use crate::seedwing::policy::static_config;
use crate::seedwing::policy::types;
use serde_json::Value;

struct Export;

impl StaticEvaluator for Export {
    fn run() -> String {
        let policies = [];
        let data = [];

        let policy_config = static_config::policy_config();
        let policy = policy_config.policy.trim();
        let policy_name = policy_config.policy_name.trim();

        let Some(input_str) = std::env::args().nth(4) else {
            return "Error: input must be specified as an argument".to_string();
        };
        let json_value: Value = serde_json::from_str(&input_str).unwrap();
        let input: types::RuntimeValue = json_value.into();

        let result = engine::eval(&policies, &data, &policy, &policy_name, &input);

        let result_context = result.unwrap();
        if result_context.severity != types::Severity::None {
            format!("{:?}: {}", result_context.severity, result_context.reason)
        } else {
            format!("Ok: {}", result_context.reason)
        }
        // TODO: should we have a command line argument that allows for details
        // to be printed?
        //let _evaluation_result = result_context.evaluation_result;
    }
}

impl From<Value> for types::RuntimeValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => types::RuntimeValue::Null,
            Value::String(value) => types::RuntimeValue::String(value.to_string().into()),
            Value::Number(number) => {
                if number.is_f64() {
                    types::RuntimeValue::Decimal(number.as_f64().unwrap_or(0.0))
                } else {
                    types::RuntimeValue::Integer(number.as_i64().unwrap_or(0))
                }
            }
            Value::Bool(value) => types::RuntimeValue::Boolean(value),
            Value::Array(list) => {
                let mut values: Vec<types::BaseValue> = Vec::with_capacity(list.len());
                for item in list {
                    let rt_value = item;
                    values.push(rt_value.into());
                }
                types::RuntimeValue::List(values)
            }
            Value::Object(obj) => {
                let mut list = Vec::with_capacity(obj.len());
                for (key, value) in obj {
                    list.push(types::Object {
                        key,
                        value: value.into(),
                    });
                }
                types::RuntimeValue::Object(list)
            }
        }
    }
}

impl From<Value> for types::ObjectValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => types::ObjectValue::Null,
            Value::String(value) => types::ObjectValue::String(value.to_string().into()),
            Value::Number(number) => {
                if number.is_f64() {
                    types::ObjectValue::Decimal(number.as_f64().unwrap_or(0.0))
                } else {
                    types::ObjectValue::Integer(number.as_i64().unwrap_or(0))
                }
            }
            Value::Bool(value) => types::ObjectValue::Boolean(value),
            Value::Array(list) => {
                let mut values: Vec<types::BaseValue> = Vec::with_capacity(list.len());
                for item in list {
                    let rt_value = item;
                    values.push(rt_value.into());
                }
                types::ObjectValue::List(values)
            }
            //Value::Octets(value) => types::ObjectValue::Octets(value.to_vec()),
            Value::Object(_) => types::ObjectValue::Null,
        }
    }
}

impl From<Value> for types::BaseValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => types::BaseValue::Null,
            Value::String(value) => types::BaseValue::String(value.to_string().into()),
            Value::Number(number) => {
                if number.is_f64() {
                    types::BaseValue::Decimal(number.as_f64().unwrap_or(0.0))
                } else {
                    types::BaseValue::Integer(number.as_i64().unwrap_or(0))
                }
            }
            Value::Bool(value) => types::BaseValue::Boolean(value),
            _ => types::BaseValue::Null,
        }
    }
}

impl From<String> for types::RuntimeValue {
    fn from(inner: String) -> Self {
        Self::String(inner.into())
    }
}

//export_static_evaluator!(Export);
