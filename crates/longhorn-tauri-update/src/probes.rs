use longhorn_update::{OutstandingWork, QuiescenceKind, QuiescenceProbe};

/// A probe over any count a host can report.
///
/// The concrete Longhorn hosts expose their outstanding work as counts —
/// `TransferCoordinator::session_count`, an operation authority's active
/// records — so the probe is a closure over one of those rather than a
/// bespoke type per subsystem. Reading the count at probe time, not at
/// construction, is the point: a receipt taken a second ago is not an answer
/// to "is it safe to restart now".
pub struct CountingProbe<F> {
    kind: QuiescenceKind,
    count: F,
}

impl<F> CountingProbe<F>
where
    F: Fn() -> usize,
{
    /// Records a probe reporting `kind` from `count`.
    pub const fn new(kind: QuiescenceKind, count: F) -> Self {
        Self { kind, count }
    }
}

impl<F> QuiescenceProbe for CountingProbe<F>
where
    F: Fn() -> usize,
{
    fn outstanding(&self) -> Option<OutstandingWork> {
        let count = (self.count)();
        (count > 0).then(|| OutstandingWork::new(self.kind, count))
    }
}

/// Reports uncommitted transfer sessions.
///
/// Wrap `TransferCoordinator::session_count`. A session that has been
/// admitted but not committed or aborted is the case that makes a restart
/// destructive rather than merely inconvenient.
#[must_use]
pub fn transfer_session_probe<F>(count: F) -> CountingProbe<F>
where
    F: Fn() -> usize,
{
    CountingProbe::new(QuiescenceKind::OpenTransferSession, count)
}

/// Reports running async operations.
///
/// Wrap the length of an operation authority's active records.
#[must_use]
pub fn operation_probe<F>(count: F) -> CountingProbe<F>
where
    F: Fn() -> usize,
{
    CountingProbe::new(QuiescenceKind::RunningOperation, count)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn a_zero_count_reports_nothing_outstanding() {
        let probe = CountingProbe::new(QuiescenceKind::PendingFlush, || 0);

        assert_eq!(probe.outstanding(), None);
    }

    #[test]
    fn the_count_is_read_at_probe_time_not_at_construction() {
        // The whole value of the interlock is that it answers about *now*.
        // A probe that captured a count when it was built would authorise a
        // restart against a stale reading.
        let sessions = AtomicUsize::new(2);
        let probe = transfer_session_probe(|| sessions.load(Ordering::Relaxed));

        assert_eq!(
            probe.outstanding(),
            Some(OutstandingWork::new(QuiescenceKind::OpenTransferSession, 2))
        );

        sessions.store(0, Ordering::Relaxed);
        assert_eq!(probe.outstanding(), None);
    }

    #[test]
    fn each_helper_reports_its_own_kind() {
        assert_eq!(
            operation_probe(|| 3).outstanding(),
            Some(OutstandingWork::new(QuiescenceKind::RunningOperation, 3))
        );
        assert_eq!(
            transfer_session_probe(|| 1).outstanding(),
            Some(OutstandingWork::new(QuiescenceKind::OpenTransferSession, 1))
        );
    }
}
