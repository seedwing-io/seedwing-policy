use seedwing_policy_engine::runtime::config::EvalConfig;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use toml::Table;

#[derive(Debug, Deserialize)]
pub struct Config {
    policy: Option<PolicyConfig>,
    data: Option<DataConfig>,
    config: Option<Table>,
}

impl Config {
    pub fn policy_directories(&self, relative_to: &Path) -> Vec<PathBuf> {
        if let Some(policy) = &self.policy {
            policy.directories(relative_to)
        } else {
            Vec::default()
        }
    }

    pub fn data_directories(&self, relative_to: &Path) -> Vec<PathBuf> {
        if let Some(data) = &self.data {
            data.directories(relative_to)
        } else {
            Vec::default()
        }
    }

    pub fn eval_config(&self) -> EvalConfig {
        if let Some(config) = &self.config {
            toml::Value::Table(config.clone()).into()
        } else {
            EvalConfig::default()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PolicyConfig {
    dirs: Vec<PathBuf>,
}

impl PolicyConfig {
    pub fn directories(&self, relative_to: &Path) -> Vec<PathBuf> {
        self.dirs.iter().map(|dir| relative_to.join(dir)).collect()
    }
}

#[derive(Debug, Deserialize)]
pub struct DataConfig {
    dirs: Vec<PathBuf>,
}

impl DataConfig {
    pub fn directories(&self, relative_to: &Path) -> Vec<PathBuf> {
        self.dirs.iter().map(|dir| relative_to.join(dir)).collect()
    }
}
