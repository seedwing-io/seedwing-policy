use crate::runtime::TypeName;
use std::collections::HashMap;
use std::time::Duration;

pub struct Statistics {
    stats: HashMap<TypeName, TypeStats>,
}

impl Statistics {
    pub fn record(&mut self, name: TypeName, elapsed: Duration) {
        if let Some(stats) = self.stats.get_mut(&name) {
            stats.record(elapsed);
        } else {
            let stats = TypeStats::new(elapsed);
            self.stats.insert(name, stats);
        }
    }
}

pub struct TypeStats {
    invocations: u64,
    mean_execution_time: Duration,
}

impl TypeStats {
    pub fn new(initial: Duration) -> Self {
        Self {
            invocations: 1,
            mean_execution_time: initial,
        }
    }

    fn record(&mut self, elapsed: Duration) {
        if let Ok(new_mean) = (((self.invocations as u128 * self.mean_execution_time.as_millis())
            + elapsed.as_millis())
            / self.invocations as u128
            + 1)
        .try_into()
        {
            self.mean_execution_time = Duration::from_millis(new_mean);
            self.invocations += 1
        } else {
            // just restart
            self.mean_execution_time = elapsed;
            self.invocations = 1
        }
    }
}
