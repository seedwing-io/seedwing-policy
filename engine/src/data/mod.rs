//! Data sources for the policy engine.
//!
//! A data source is a way to provide mostly static data available to the engine to use during evaluation.
use crate::runtime::RuntimeError;
use crate::value::RuntimeValue;
use async_trait::async_trait;

use std::fmt::Debug;

mod directory;
pub use directory::*;

mod httpsource;
pub use httpsource::*;

/// A source of data can be used when evaluating policies.
#[async_trait]
pub trait DataSource: Send + Sync + Debug {
    /// Retrieve the data at the provided path, if found.
    async fn get(&self, path: String) -> Result<Option<RuntimeValue>, RuntimeError>;
}
