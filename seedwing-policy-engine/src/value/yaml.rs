use crate::value::serde::{to_value, Error};
use crate::value::{Object, RuntimeValue};
use serde_yaml::Value as YamlValue;
use std::rc::Rc;

fn to_key(k: YamlValue) -> String {
    match k {
        YamlValue::Null => "null".to_string(),
        YamlValue::Bool(v) => v.to_string(),
        YamlValue::Number(v) => v.to_string(),
        YamlValue::String(v) => v,
        YamlValue::Sequence(v) => v.into_iter().map(|s| to_key(s)).collect(),
        YamlValue::Mapping(v) => v
            .into_iter()
            .map(|(k, v)| format!("{}/{}", to_key(k), to_key(v)))
            .collect(),
        YamlValue::Tagged(v) => {
            format!("{}:{}", v.tag, to_key(v.value))
        }
    }
}

impl From<YamlValue> for RuntimeValue {
    fn from(value: YamlValue) -> Self {
        match value {
            YamlValue::Null => RuntimeValue::Null,
            YamlValue::Bool(inner) => RuntimeValue::Boolean(inner),
            YamlValue::Number(inner) => {
                if inner.is_f64() {
                    RuntimeValue::Decimal(inner.as_f64().unwrap())
                } else if inner.is_i64() {
                    RuntimeValue::Integer(inner.as_i64().unwrap())
                } else {
                    todo!("u64 is needed, I guess")
                }
            }
            YamlValue::String(inner) => RuntimeValue::String(inner),
            YamlValue::Sequence(inner) => RuntimeValue::List(
                inner
                    .into_iter()
                    .map(|e| Rc::new(RuntimeValue::from(e)))
                    .collect(),
            ),
            YamlValue::Mapping(inner) => {
                let fields = inner
                    .into_iter()
                    .map(|(k, v)| (to_key(k), Rc::new(RuntimeValue::from(v))))
                    .collect();

                RuntimeValue::Object(Object { fields })
            }
            YamlValue::Tagged(inner) => {
                let mut o = Object::new();
                o.set(inner.tag.to_string(), inner.value);
                RuntimeValue::Object(o)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::value::test::assert_yaml;

    #[test]
    fn test_yaml() {
        assert_yaml(|y| serde_yaml::from_str::<serde_yaml::Value>(y).map(|v| v.into()));
    }
}
