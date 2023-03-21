use std::path::PathBuf;

#[derive(Debug)]
pub enum CliError {
    Unknown,
    Config(ConfigError),
}

impl From<()> for CliError {
    fn from(_value: ()) -> Self {
        Self::Unknown
    }
}

impl From<ConfigError> for CliError {
    fn from(inner: ConfigError) -> Self {
        CliError::Config(inner)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),
    #[error("Not readable")]
    NotReadable,
    #[error("Invalid format")]
    InvalidFormat,
    #[error("Parser error: {0}")]
    Deserialization(toml::de::Error),
}

impl From<toml::de::Error> for ConfigError {
    fn from(inner: toml::de::Error) -> Self {
        Self::Deserialization(inner)
    }
}
