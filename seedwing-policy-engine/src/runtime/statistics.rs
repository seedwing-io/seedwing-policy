use crate::runtime::monitor::Completion;
use crate::runtime::{Output, TypeName};
use num_integer::Roots;
use rand::rngs::ThreadRng;
use rand::Rng;
use serde::Serialize;
use std::collections::HashMap;
use std::env::var;
use std::time::Duration;

pub struct Statistics<const N: usize = 100> {
    stats: HashMap<TypeName, TypeStats<N>>,
}

impl<const N: usize> Default for Statistics<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Statistics<N> {
    pub fn new() -> Self {
        Self {
            stats: Default::default(),
        }
    }

    pub fn record(&mut self, name: TypeName, elapsed: Duration, completion: &Completion) {
        if let Some(stats) = self.stats.get_mut(&name) {
            stats.record(elapsed, completion);
        } else {
            let stats = TypeStats::new(elapsed, completion);
            self.stats.insert(name, stats);
        }
    }

    pub fn snapshot(&self) -> Vec<Snapshot> {
        self.stats
            .iter()
            .map(|(name, stats)| stats.snapshot(name))
            .collect()
    }
}

#[derive(Clone)]
pub struct TypeStats<const N: usize> {
    invocations: u64,
    satisfied_invocations: u64,
    unsatisfied_invocations: u64,
    error_invocations: u64,
    samples: [u128; N],
    num_samples: u8,
}

impl<const N: usize> TypeStats<N> {
    pub fn new(elapsed: Duration, completion: &Completion) -> Self {
        let mut this = Self {
            invocations: 0,
            satisfied_invocations: 0,
            unsatisfied_invocations: 0,
            error_invocations: 0,
            samples: [0; N],
            num_samples: 0,
        };

        this.record(elapsed, completion);

        this
    }

    fn record(&mut self, elapsed: Duration, completion: &Completion) {
        self.invocations += 1;
        match completion {
            Completion::Output(output) => match output {
                Output::None => self.unsatisfied_invocations += 1,
                _ => self.satisfied_invocations += 1,
            },
            Completion::Err(_) => {
                self.error_invocations += 1;
            }
        }

        if (self.num_samples as usize) < N {
            self.samples[self.num_samples as usize] = elapsed.as_nanos();
            self.num_samples += 1
        } else {
            let num = rand::thread_rng().gen_range(0..100) as usize;
            self.samples[num] = elapsed.as_nanos()
        }
    }

    fn snapshot(&self, name: &TypeName) -> Snapshot {
        Snapshot {
            name: name.as_type_str(),
            mean: self.mean(),
            median: self.median(),
            stddev: self.stddev(),
            invocations: self.invocations,
            satisfied_invocations: self.satisfied_invocations,
            unsatisfied_invocations: self.unsatisfied_invocations,
            error_invocations: self.error_invocations,
        }
    }

    fn mean(&self) -> u128 {
        self.samples[0..self.num_samples as usize]
            .iter()
            .sum::<u128>()
            / self.num_samples as u128
    }

    fn median(&self) -> u128 {
        let mut samples = [0; N];
        samples.clone_from_slice(&self.samples);
        let mut samples = &mut samples[0..self.num_samples as usize];
        samples.sort();
        samples[samples.len() / 2]
    }

    fn stddev(&self) -> u128 {
        let mean = self.mean();

        let variance = self.samples[0..self.num_samples as usize]
            .iter()
            .map(|value| {
                let diff = if (mean > *value) {
                    mean - (*value)
                } else {
                    *value - mean
                };
                diff * diff
            })
            .sum::<u128>()
            / self.num_samples as u128;

        variance.sqrt()
    }
}

#[derive(Serialize)]
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
