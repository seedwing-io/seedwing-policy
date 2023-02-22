use serde::{Deserialize, Serialize};

#[cfg(feature = "monitor")]
pub mod monitor;
#[cfg(feature = "monitor")]
pub mod prometheus;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
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
