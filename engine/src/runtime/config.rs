use crate::value::RuntimeValue;
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
        obj: &serde_json::Map<String, serde_json::Value>,
    ) {
        for (k, v) in obj.iter() {
            let mut prefix = prefix.clone();
            if !prefix.is_empty() {
                prefix.push('.');
            }
            prefix.push_str(k.as_str());
            match v {
                serde_json::Value::Object(obj) => {
                    Self::parse_json_object(prefix, config, obj);
                }
                _ => Self::parse_json_other(prefix, config, v),
            }
        }
    }

    fn parse_json_other(
        prefix: String,
        config: &mut HashMap<String, ConfigValue>,
        val: &serde_json::Value,
    ) {
        match val {
            serde_json::Value::Bool(val) => {
                config.insert(prefix, (*val).into());
            }
            serde_json::Value::Number(val) => {
                if let Some(val) = val.as_i64() {
                    config.insert(prefix, val.into());
                } else if let Some(val) = val.as_f64() {
                    config.insert(prefix, val.into());
                }
            }
            serde_json::Value::String(val) => {
                config.insert(prefix, val.clone().into());
            }
            _ => {
                // ignore
            }
        }
    }

    fn parse_toml_table(
        prefix: String,
        config: &mut HashMap<String, ConfigValue>,
        table: &toml::Table,
    ) {
        for (k, v) in table.iter() {
            let mut prefix = prefix.clone();
            if !prefix.is_empty() {
                prefix.push('.');
            }
            prefix.push_str(k.as_str());
            match v {
                toml::Value::Table(table) => {
                    Self::parse_toml_table(prefix, config, table);
                }
                _ => Self::parse_toml_other(prefix, config, v),
            }
        }
    }

    fn parse_toml_other(
        prefix: String,
        config: &mut HashMap<String, ConfigValue>,
        val: &toml::Value,
    ) {
        match val {
            toml::Value::String(val) => {
                config.insert(prefix, val.clone().into());
            }
            toml::Value::Integer(val) => {
                config.insert(prefix, (*val).into());
            }
            toml::Value::Float(val) => {
                config.insert(prefix, (*val).into());
            }
            toml::Value::Boolean(val) => {
                config.insert(prefix, (*val).into());
            }
            _ => {
                // nothing
            }
        }
    }
}

impl From<serde_json::Value> for EvalConfig {
    fn from(json: serde_json::Value) -> Self {
        let prefix: String = "".into();
        let mut config = HashMap::new();
        if let serde_json::Value::Object(obj) = &json {
            EvalConfig::parse_json_object(prefix, &mut config, obj)
        }
        Self(config)
    }
}

impl From<toml::Value> for EvalConfig {
    fn from(toml: toml::Value) -> Self {
        let prefix: String = "".into();
        let mut config = HashMap::new();
        if let toml::Value::Table(table) = &toml {
            EvalConfig::parse_toml_table(prefix, &mut config, table)
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
