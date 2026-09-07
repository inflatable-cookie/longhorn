//! Call-bound cancellation observation.

use std::fmt;
use std::sync::Arc;

/// Host-owned source of one call's cancellation state.
pub trait CancellationSource: Send + Sync {
    /// Reports whether the exact bound call is cancelled.
    fn is_cancelled(&self) -> bool;
}

/// Read-only cancellation observation passed to one admitted callback.
#[derive(Clone)]
pub struct Cancellation {
    source: Arc<dyn CancellationSource>,
}

impl Cancellation {
    /// Binds a host-owned cancellation source to one callback invocation.
    #[must_use]
    pub fn new(source: Arc<dyn CancellationSource>) -> Self {
        Self { source }
    }

    /// Reports whether the bound call is cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.source.is_cancelled()
    }
}

impl fmt::Debug for Cancellation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Cancellation")
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}
