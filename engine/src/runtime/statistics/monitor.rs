use crate::runtime::monitor::Completion;
use crate::runtime::statistics::Snapshot;
use crate::runtime::PatternName;
use num_integer::Roots;

use rand::Rng;

use std::collections::HashMap;

use crate::lang::Severity;
use std::time::Duration;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;

#[cfg(feature = "prometheus")]
use crate::runtime::statistics::prometheus::PrometheusStats;

pub struct Statistics<const N: usize = 100> {
    stats: HashMap<PatternName, PatternStats<N>>,
    subscribers: Mutex<Vec<Subscriber>>,

    #[cfg(feature = "prometheus")]
    prom_stats: PrometheusStats,
}

#[cfg(not(feature = "prometheus"))]
impl<const N: usize> Default for Statistics<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Statistics<N> {
    #[cfg(not(feature = "prometheus"))]
    pub fn new() -> Self {
        Self {
            stats: Default::default(),
            subscribers: Default::default(),
        }
    }

    #[cfg(feature = "prometheus")]
    pub fn new(registry: &'static prometheus::Registry) -> Self {
        Self {
            stats: Default::default(),
            subscribers: Default::default(),
            prom_stats: PrometheusStats::new(registry),
        }
    }

    pub async fn record(&mut self, name: PatternName, elapsed: Duration, completion: &Completion) {
        let snapshot = if let Some(stats) = self.stats.get_mut(&name) {
            stats.record(elapsed, completion);
            stats.snapshot(&name)
        } else {
            let stats = PatternStats::new(elapsed, completion);
            let snapshot = stats.snapshot(&name);
            self.stats.insert(name.clone(), stats);
            snapshot
        };

        #[cfg(feature = "prometheus")]
        self.prom_stats.record(&name, elapsed, completion);

        self.fanout(snapshot).await;
    }

    pub fn snapshot(&self) -> Vec<Snapshot> {
        self.stats
            .iter()
            .map(|(name, stats)| stats.snapshot(name))
            .collect()
    }

    pub async fn subscribe(&self, path: String) -> Receiver<Snapshot> {
        let (sender, receiver) = channel(50);
        self.subscribers.lock().await.push(Subscriber {
            path,
            sender,
            disconnected: false,
        });
        receiver
    }

    async fn fanout(&self, snapshot: Snapshot) {
        for subscriber in self
            .subscribers
            .lock()
            .await
            .iter_mut()
            .filter(|sub| sub.interested_in(snapshot.name.clone()))
        {
            if let Err(err) = subscriber.sender.try_send(snapshot.clone()) {
                match err {
                    TrySendError::Full(_) => {
                        // ehhh
                    }
                    TrySendError::Closed(_) => subscriber.disconnected = true,
                }
            }
        }

        let mut locked = self.subscribers.lock().await;
        let live_subscribers = locked.iter().filter(|e| !e.disconnected).cloned().collect();
        *locked = live_subscribers
    }
}

#[derive(Clone)]
pub struct PatternStats<const N: usize> {
    invocations: u64,
    satisfied_invocations: u64,
    unsatisfied_invocations: u64,
    error_invocations: u64,
    samples: [u128; N],
    num_samples: u8,
}

impl<const N: usize> PatternStats<N> {
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
            Completion::Ok {
                severity,
                output: _,
            } => match severity {
                Severity::Error => self.unsatisfied_invocations += 1,
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

    fn snapshot(&self, name: &PatternName) -> Snapshot {
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
        let samples = &mut samples[0..self.num_samples as usize];
        samples.sort();
        samples[samples.len() / 2]
    }

    fn stddev(&self) -> u128 {
        let mean = self.mean();

        let variance = self.samples[0..self.num_samples as usize]
            .iter()
            .map(|value| {
                let diff = if mean > *value {
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

#[derive(Clone)]
pub struct Subscriber {
    path: String,
    sender: Sender<Snapshot>,
    disconnected: bool,
}

impl Subscriber {
    pub fn interested_in(&self, name: String) -> bool {
        name.starts_with(&self.path)
    }
}
