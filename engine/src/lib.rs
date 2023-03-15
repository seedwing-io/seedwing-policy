#![doc = include_str!("../../README.md")]
#![deny(warnings)]
//#![warn(missing_docs)]

mod core;
pub mod data;
pub mod lang;
mod package;
pub mod runtime;
pub mod value;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const TAG: Option<&str> = option_env!("TAG");

/// Current version of Seedwing
pub fn version() -> &'static str {
    TAG.unwrap_or(VERSION)
}
