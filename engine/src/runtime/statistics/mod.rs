//! Statistics for policy evaluation.
use serde::{Deserialize, Serialize};

#[cfg(feature = "monitor")]
pub mod monitor;
#[cfg(feature = "monitor")]
pub mod prometheus;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Snapshot {
    pub name: String,
    pub mean: u128,
    pub median: u128,
    pub stddev: u128,
    pub invocations: u64,
    pub satisfied_invocations: u64,
    pub unsatisfied_invocations: u64,
    pub error_invocations: u64,
}

impl PartialEq for Snapshot {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(&other.name)
    }
}
