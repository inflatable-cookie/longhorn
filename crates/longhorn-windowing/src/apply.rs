use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::WindowId;

use crate::{
    ApplyGeneration, DeferredSettlement, LiveWindow, PlannedWindowOperation, WindowDiffError,
    WindowDiffInput, WindowDiffReceipt, WindowOperation, WindowOperationKind, plan_window_diff,
};

/// Host operations and evidence used by the ordered apply loop.
pub trait WindowApplyHost {
    /// Host-specific per-operation receipt.
    type Attempt;
    /// Complete native readback and its live windows.
    type Observation: AsRef<[LiveWindow]>;
    /// Planning or generation admission failure.
    type ApplyError: From<WindowDiffError>;
    /// Failure to obtain complete readback.
    type ReadbackError;

    /// Admits this generation before any operation runs.
    fn begin(&mut self, generation: ApplyGeneration) -> Result<(), Self::ApplyError>;
    /// Executes an operation and reports whether it succeeded.
    fn execute(&mut self, operation: &PlannedWindowOperation) -> (Self::Attempt, bool);
    /// Records a dependency skip without issuing native calls.
    fn skip(
        &self,
        operation: &PlannedWindowOperation,
        blocked_by: WindowOperationKind,
    ) -> Self::Attempt;
    /// Reads a complete fresh observation.
    fn readback(&mut self) -> Result<Self::Observation, Self::ReadbackError>;
}

/// Fresh native readback and the diff against the original desired state.
pub type WindowApplyReadback<O, E> = Result<(O, Result<WindowDiffReceipt, WindowDiffError>), E>;

/// Shared ordered apply result before host-specific receipt projection.
#[derive(Debug)]
pub struct WindowApplyRun<A, O, E> {
    /// Original diff, with every operation in execution order.
    pub plan: WindowDiffReceipt,
    /// One receipt per planned operation.
    pub attempts: Vec<A>,
    /// Identities successfully created in this generation.
    pub created: BTreeSet<WindowId>,
    /// Readback and convergence evidence.
    pub readback: WindowApplyReadback<O, E>,
}

/// Completed run or a failure before native apply began.
pub type WindowApplyResult<H> = Result<
    WindowApplyRun<
        <H as WindowApplyHost>::Attempt,
        <H as WindowApplyHost>::Observation,
        <H as WindowApplyHost>::ReadbackError,
    >,
    <H as WindowApplyHost>::ApplyError,
>;

/// Executes an ordered nontransactional apply and checks fresh convergence.
///
/// # Errors
/// Planning or generation admission can fail before any operation executes.
pub fn apply_window_diff<H: WindowApplyHost>(
    host: &mut H,
    input: WindowDiffInput,
    settlement: &DeferredSettlement,
) -> WindowApplyResult<H> {
    let plan = plan_window_diff(&input)?;
    host.begin(plan.generation())?;

    let live_ids: BTreeMap<_, _> = input
        .live_windows()
        .iter()
        .filter_map(|window| Some((window.transport_handle(), window.window_id()?)))
        .collect();
    let mut blocked = BTreeMap::new();
    let mut created = BTreeSet::new();
    let mut attempts = Vec::with_capacity(plan.operations().len());
    for planned in plan.operations() {
        let operation = planned.operation();
        let window_id = operation.window_id();
        let kind = operation.kind();
        let attempt = if let Some(blocked_by) = blocked.get(window_id) {
            host.skip(planned, *blocked_by)
        } else {
            let (attempt, succeeded) = host.execute(planned);
            if !succeeded {
                blocked.insert(window_id, kind);
                // Retag was supposed to free the source id for a later create.
                if let WindowOperation::Retag {
                    transport_handle, ..
                } = operation
                    && let Some(previous_id) = live_ids.get(transport_handle)
                {
                    blocked.insert(*previous_id, kind);
                }
            } else if kind == WindowOperationKind::Create {
                created.insert(window_id.clone());
            }
            attempt
        };
        attempts.push(attempt);
    }

    let readback = host.readback().map(|observation| {
        let input = input.with_live_windows(observation.as_ref().iter().cloned());
        let convergence = plan_window_diff(&input).map(|plan| plan.without_deferred(settlement));
        (observation, convergence)
    });
    Ok(WindowApplyRun {
        plan,
        attempts,
        created,
        readback,
    })
}

/// Rejection of a newly created window, with the outcome of discarding it.
#[derive(Debug)]
pub struct RejectedWindow<W, E, C> {
    /// Reason why the window could not be accepted.
    pub reason: E,
    /// Native cleanup result, owning the still-tracked window if cleanup failed.
    pub cleanup: Result<(), (W, C)>,
}

/// Accepts a freshly created native window or immediately discards it.
///
/// A successful acceptance transfers responsibility to the registry. A
/// rejection must leave the registry unchanged and the caller must report
/// both the rejection and any native cleanup failure. The caller retains the
/// returned native window for cleanup retry if the host refused to discard it.
pub fn accept_created_window<W, T, E, C>(
    window: W,
    accept: impl FnOnce(&W) -> Result<T, E>,
    discard: impl FnOnce(&W) -> Result<(), C>,
) -> Result<T, RejectedWindow<W, E, C>> {
    accept(&window).map_err(|reason| RejectedWindow {
        reason,
        cleanup: discard(&window).map_err(|error| (window, error)),
    })
}
