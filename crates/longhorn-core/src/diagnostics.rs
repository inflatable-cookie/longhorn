//! Process-wide best-effort failure diagnostics.
//!
//! Longhorn deliberately tolerates some failures — event-emit hints, adapter
//! teardown, terminal journal cleanup — because the owning operation must not
//! fail with them. This seam makes that class observable: composition owners
//! install one sink at startup, and every tolerated failure reports through
//! it. With no sink installed the behavior is exactly the historical silent
//! tolerance.

use std::sync::{Arc, OnceLock};

/// Observer for failures Longhorn tolerates by design.
pub trait BestEffortDiagnostics: Send + Sync {
    /// Records one tolerated failure. `area` is a stable dotted site name;
    /// `detail` is a human-readable diagnostic.
    fn best_effort_failure(&self, area: &'static str, detail: &str);
}

static SINK: OnceLock<Arc<dyn BestEffortDiagnostics>> = OnceLock::new();

/// Installs the process-wide diagnostics sink. The first installation wins;
/// later calls return `false` and change nothing.
pub fn install_best_effort_diagnostics(sink: Arc<dyn BestEffortDiagnostics>) -> bool {
    SINK.set(sink).is_ok()
}

/// Reports one tolerated failure to the installed sink, if any.
pub fn report_best_effort_failure(area: &'static str, detail: impl core::fmt::Display) {
    if let Some(sink) = SINK.get() {
        sink.best_effort_failure(area, &detail.to_string());
    }
}
