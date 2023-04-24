//! Data sources for the policy engine.
//!
//! A data source is a way to provide mostly static data available to the engine to use during evaluation.
use crate::runtime::RuntimeError;
use crate::value::RuntimeValue;

use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

/// A source of data can be used when evaluating policies.
pub trait DataSource: Send + Sync + Debug {
    /// Retrieve the data at the provided path, if found.
    fn get(&self, path: &str) -> Result<Option<RuntimeValue>, RuntimeError>;
}

/// A source of data read from a directory.
///
/// The path parameter is used to locate the source file within the root directory.
#[derive(Debug)]
pub struct DirectoryDataSource {
    root: PathBuf,
}

impl DirectoryDataSource {
    /// Create a directory data source based on the root directory parameter.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl DataSource for DirectoryDataSource {
    fn get(&self, path: &str) -> Result<Option<RuntimeValue>, RuntimeError> {
        let target = self.root.join(path);

        if target.exists() {
            if target.is_dir() {
                Err(RuntimeError::FileUnreadable(target))
            } else if let Some(name) = target.file_name() {
                log::info!("read from file: {:?}", name);
                if name.to_string_lossy().ends_with(".json") {
                    // parse as JSON
                    if let Ok(file) = File::open(target.clone()) {
                        let json: Result<serde_json::Value, _> = serde_json::from_reader(file);
                        match json {
                            Ok(json) => Ok(Some(json.into())),
                            Err(e) => Err(RuntimeError::JsonError(target, e)),
                        }
                    } else {
                        Err(RuntimeError::FileUnreadable(target))
                    }
                } else if name.to_string_lossy().ends_with(".yaml")
                    || name.to_string_lossy().ends_with(".yml")
                {
                    // parse as YAML
                    if let Ok(file) = File::open(target.clone()) {
                        let yaml: Result<serde_json::Value, _> = serde_yaml::from_reader(file);
                        match yaml {
                            Ok(yaml) => Ok(Some(yaml.into())),
                            Err(e) => Err(RuntimeError::YamlError(target, e)),
                        }
                    } else {
                        Err(RuntimeError::FileUnreadable(target))
                    }
                } else if let Ok(mut file) = File::open(target.clone()) {
                    // just octets
                    let mut octets = Vec::new();
                    file.read_to_end(&mut octets)
                        .map_err(|_| RuntimeError::FileUnreadable(target))?;
                    Ok(Some(RuntimeValue::Octets(octets)))
                } else {
                    Err(RuntimeError::FileUnreadable(target))
                }
            } else {
                Ok(None)
            }
        } else {
            log::error!("{:?} not found", target);
            Ok(None)
        }
    }
}
