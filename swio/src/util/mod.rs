use crate::cli::InputType;
use is_terminal::IsTerminal;
use seedwing_policy_engine::value::RuntimeValue;
use serde_yaml::Error as YamlError;
use std::io::stdin;
use std::path::PathBuf;
use tokio::fs;

pub mod eval;

pub async fn load_values(
    typ: InputType,
    inputs: Vec<PathBuf>,
) -> Result<Vec<RuntimeValue>, std::io::Error> {
    if !inputs.is_empty() {
        let mut values = Vec::new();
        for input in inputs.iter() {
            let data = fs::read(input).await?;

            match typ {
                InputType::Json => {
                    let value: serde_json::Value = serde_json::from_slice(&data)?;
                    values.push(value.into());
                }
                InputType::Yaml => {
                    let value: serde_json::Value = serde_yaml::from_slice(&data)
                        .map_err(YamlError::from)
                        .unwrap();
                    values.push(value.into());
                }
            }
        }
        Ok(values)
    } else {
        if stdin().is_terminal() {
            println!("Enter input value, ^D to finish");
        }
        match typ {
            InputType::Json => {
                let value: serde_json::Value = serde_json::from_reader(stdin())?;
                Ok(vec![value.into()])
            }
            InputType::Yaml => {
                let value: serde_json::Value = serde_yaml::from_reader(stdin())
                    .map_err(YamlError::from)
                    .unwrap();
                Ok(vec![value.into()])
            }
        }
    }
}
