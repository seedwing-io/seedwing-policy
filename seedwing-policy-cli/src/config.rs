use std::path::PathBuf;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    policies: String,
    requires: String,
}

// todo policy can be a path to a file or an inline document.
//todo json == yaml, remove the type arg


impl Config {
    pub fn policy(&self) -> PathBuf {
        PathBuf::from("someDir")
    }
}