use longhorn_update::{OutstandingWork, QuiescenceKind, QuiescenceProbe};

/// Reports the window host's own outstanding work to the restart interlock.
///
/// Contract 020 requires that "the host reports its own outstanding work to
/// the restart interlock". `QuiescenceProbe` is a plain pure trait in
/// `longhorn-update`, so this needed no host-specific mechanism.
///
/// The count is read at probe time. A receipt taken a second ago is not an
/// answer to "is it safe to restart now".
pub struct GpuiWindowQuiescenceProbe<F> {
    outstanding: F,
}

impl<F> GpuiWindowQuiescenceProbe<F>
where
    F: Fn() -> usize,
{
    /// Records a probe over the host's outstanding capture and flush count.
    pub const fn new(outstanding: F) -> Self {
        Self { outstanding }
    }
}

impl<F> QuiescenceProbe for GpuiWindowQuiescenceProbe<F>
where
    F: Fn() -> usize,
{
    fn outstanding(&self) -> Option<OutstandingWork> {
        let count = (self.outstanding)();
        (count > 0).then(|| OutstandingWork::new(QuiescenceKind::PendingFlush, count))
    }
}
