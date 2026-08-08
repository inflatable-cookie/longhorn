use std::time::Instant;

use longhorn_windowing::MonotonicMillis;

/// Process-local monotonic time source.
pub trait WindowLifecycleClock: Send + Sync {
    /// Returns milliseconds from the clock's arbitrary epoch.
    fn now(&self) -> MonotonicMillis;
}

/// `Instant`-backed process clock.
pub struct ProcessMonotonicClock {
    epoch: Instant,
}

impl ProcessMonotonicClock {
    /// Starts a new arbitrary process-local epoch.
    #[must_use]
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
        }
    }
}

impl Default for ProcessMonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowLifecycleClock for ProcessMonotonicClock {
    fn now(&self) -> MonotonicMillis {
        let elapsed = self.epoch.elapsed().as_millis();
        MonotonicMillis::new(u64::try_from(elapsed).unwrap_or(u64::MAX))
    }
}
