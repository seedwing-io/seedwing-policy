use crate::value::RuntimeValue;
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct EvalConfig(HashMap<String, ConfigValue>);

impl EvalConfig {
    pub fn get(&self, key: &String) -> Option<&ConfigValue> {
        self.0.get(key)
    }
}

impl EvalConfig {
    fn parse_json_object(
        prefix: String,
        config: &mut HashMap<String, ConfigValue>,
        obj: &Map<String, Value>,
    ) {
        for (k, v) in obj.iter() {
            let mut prefix = prefix.clone();
            if !prefix.is_empty() {
                prefix.push('.');
            }
            prefix.push_str(k.as_str());
            match v {
                Value::Object(obj) => {
                    Self::parse_json_object(prefix, config, obj);
                }
                _ => Self::parse_json_other(prefix, config, v),
            }
        }
    }

    fn parse_json_other(prefix: String, config: &mut HashMap<String, ConfigValue>, val: &Value) {
        match val {
            Value::Bool(val) => {
                config.insert(prefix, (*val).into());
            }
            Value::Number(val) => {
                if let Some(val) = val.as_i64() {
                    config.insert(prefix, val.into());
                } else if let Some(val) = val.as_f64() {
                    config.insert(prefix, val.into());
                }
            }
            Value::String(val) => {
                config.insert(prefix, val.clone().into());
            }
            _ => {
                // ignore
            }
        }
    }
}

impl From<Value> for EvalConfig {
    fn from(json: Value) -> Self {
        let prefix: String = "".into();
        let mut config = HashMap::new();
        if let Value::Object(obj) = &json {
            EvalConfig::parse_json_object(prefix, &mut config, obj)
        }
        Self(config)
    }
}

#[derive(Debug, Clone)]
pub enum ConfigValue {
    Integer(i64),
    Decimal(f64),
    String(String),
    Boolean(bool),
}

impl From<i64> for ConfigValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<f64> for ConfigValue {
    fn from(value: f64) -> Self {
        Self::Decimal(value)
    }
}

impl From<String> for ConfigValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<bool> for ConfigValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<&ConfigValue> for RuntimeValue {
    fn from(value: &ConfigValue) -> Self {
        match value {
            ConfigValue::Integer(val) => (*val).into(),
            ConfigValue::Decimal(val) => (*val).into(),
            ConfigValue::String(val) => val.clone().into(),
            ConfigValue::Boolean(val) => (*val).into(),
        }
    }
}
