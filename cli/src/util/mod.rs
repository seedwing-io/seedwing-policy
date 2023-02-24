use crate::cli::InputType;
use is_terminal::IsTerminal;
use seedwing_policy_engine::value::RuntimeValue;
use serde_yaml::Error as YamlError;
use std::io::stdin;
use std::path::PathBuf;
use tokio::fs;

pub mod eval;
pub mod explain;

pub async fn load_value(
    typ: InputType,
    input: Option<PathBuf>,
) -> Result<RuntimeValue, std::io::Error> {
    if let Some(input) = input {
        let data = fs::read(input).await?;

        match typ {
            InputType::Json => {
                let value: serde_json::Value = serde_json::from_slice(&data)?;
                Ok(value.into())
            }
            InputType::Yaml => {
                let value: serde_json::Value = serde_yaml::from_slice(&data)
                    .map_err(YamlError::from)
                    .unwrap();
                Ok(value.into())
            }
        }
    } else {
        if stdin().is_terminal() {
            println!("Enter input value, ^D to finish");
        }
        match typ {
            InputType::Json => {
                let value: serde_json::Value = serde_json::from_reader(stdin())?;
                Ok(value.into())
            }
            InputType::Yaml => {
                let value: serde_json::Value = serde_yaml::from_reader(stdin())
                    .map_err(YamlError::from)
                    .unwrap();
                Ok(value.into())
            }
        }
    }
}
