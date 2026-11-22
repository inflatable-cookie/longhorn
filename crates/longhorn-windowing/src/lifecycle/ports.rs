use std::time::Instant;

use longhorn_core::WindowId;

use super::MonotonicMillis;

/// Process-local monotonic time source.
pub trait WindowLifecycleClock {
    /// Returns milliseconds from the clock's arbitrary epoch.
    fn now(&self) -> MonotonicMillis;
}

/// `Instant`-backed process clock.
#[derive(Debug)]
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

/// Consumer-owned user-close policy callback.
pub trait WindowUserCloseHandler {
    /// Receives user close without inferred desired-state mutation.
    fn user_close(&self, window_id: &WindowId) -> Result<(), String>;
}

/// Explicit no-op user-close policy.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopWindowUserCloseHandler;

impl WindowUserCloseHandler for NoopWindowUserCloseHandler {
    fn user_close(&self, _window_id: &WindowId) -> Result<(), String> {
        Ok(())
    }
}

impl<F> WindowUserCloseHandler for F
where
    F: Fn(&WindowId) -> Result<(), String>,
{
    fn user_close(&self, window_id: &WindowId) -> Result<(), String> {
        self(window_id)
    }
}
