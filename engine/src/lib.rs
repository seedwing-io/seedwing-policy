#![doc = include_str!("../../README.md")]
//#![deny(warnings)]
//#![warn(missing_docs)]

mod core;
pub mod data;
pub mod lang;
mod package;
pub mod runtime;
pub mod value;

/// Common test functionality
#[doc(hidden)]
pub mod test;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const TAG: Option<&str> = option_env!("TAG");

/// Current version of Seedwing
pub const fn version() -> &'static str {
    if let Some(tag) = TAG {
        tag
    } else {
        VERSION
    }
}
