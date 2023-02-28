use crate::runtime::monitor::Completion;
use crate::runtime::{Output, PatternName};

use std::time::Duration;

#[cfg(feature = "prometheus")]
pub(super) struct PrometheusStats {
    eval_time: prometheus::HistogramVec,
    satisfied: prometheus::CounterVec,
    unsatisfied: prometheus::CounterVec,
    error: prometheus::CounterVec,
}

#[cfg(feature = "prometheus")]
impl PrometheusStats {
    pub(super) fn new(registry: &'static prometheus::Registry) -> Self {
        Self {
            eval_time: prometheus::register_histogram_vec_with_registry!(
                "seedwing_eval_time_seconds",
                "help",
                &["name"],
                registry
            )
            .unwrap(),
            satisfied: prometheus::register_counter_vec_with_registry!(
                "seedwing_satisfied_count",
                "help",
                &["name"],
                registry
            )
            .unwrap(),
            unsatisfied: prometheus::register_counter_vec_with_registry!(
                "seedwing_unsatisfied_count",
                "help",
                &["name"],
                registry
            )
            .unwrap(),
            error: prometheus::register_counter_vec_with_registry!(
                "seedwing_error_count",
                "help",
                &["name"],
                registry
            )
            .unwrap(),
        }
    }

    pub(super) fn record(
        &mut self,
        name: &PatternName,
        elapsed: Duration,
        completion: &Completion,
    ) {
        match completion {
            Completion::Output(output) => match output {
                Output::None => self.unsatisfied.with_label_values(&[name.name()]).inc(),
                _ => self.satisfied.with_label_values(&[name.name()]).inc(),
                _ => {}
            },
            Completion::Err(_) => self.error.with_label_values(&[name.name()]).inc(),
        }

        self.eval_time
            .with_label_values(&[name.name()])
            .observe(elapsed.as_secs_f64());
    }
}
