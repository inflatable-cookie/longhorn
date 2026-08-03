//! Best-effort diagnostics seam behavior.

use std::sync::{Arc, Mutex};

use longhorn_core::{
    BestEffortDiagnostics, install_best_effort_diagnostics, report_best_effort_failure,
};

#[derive(Default)]
struct CaptureSink(Mutex<Vec<(&'static str, String)>>);

impl BestEffortDiagnostics for CaptureSink {
    fn best_effort_failure(&self, area: &'static str, detail: &str) {
        self.0.lock().unwrap().push((area, detail.to_string()));
    }
}

#[test]
fn installed_sink_observes_reports_and_first_installation_wins() {
    // Reporting without a sink is silent and must not panic.
    report_best_effort_failure("test.before-install", "dropped");

    let sink = Arc::new(CaptureSink::default());
    assert!(install_best_effort_diagnostics(sink.clone()));
    assert!(!install_best_effort_diagnostics(Arc::new(
        CaptureSink::default()
    )));

    report_best_effort_failure("test.area", std::io::Error::other("boom"));
    let captured = sink.0.lock().unwrap().clone();
    assert_eq!(captured, vec![("test.area", "boom".to_string())]);
}
