mod protected;
mod state;

use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::WindowId;

use self::{
    protected::plan_protected_primary,
    state::{plan_creates, plan_desired_state, plan_focus},
};
use super::{
    DesiredWindow, HostCapabilities, HostCapability, HostWindowHandle, LiveWindow,
    PlannedWindowOperation, WindowDiffDiagnostic, WindowDiffError, WindowDiffInput,
    WindowDiffReceipt, WindowOperation,
};

#[derive(Clone)]
pub(super) enum Binding {
    Existing(LiveWindow),
    Created,
}

impl Binding {
    pub(super) fn handle(&self) -> Option<HostWindowHandle> {
        match self {
            Self::Existing(live) => Some(live.transport_handle().clone()),
            Self::Created => None,
        }
    }
}

/// Plans a deterministic host mutation sequence without touching native state.
pub fn plan_window_diff(input: &WindowDiffInput) -> Result<WindowDiffReceipt, WindowDiffError> {
    let desired = collect_desired(input.desired_windows())?;
    let live = collect_live(input.live_windows())?;
    let mut bindings = stable_bindings(&desired, &live);
    let mut protected = BTreeSet::new();
    let mut blocked_creates = BTreeSet::new();
    let mut operations = Vec::new();
    let mut diagnostics = Vec::new();

    plan_protected_primary(
        input,
        &desired,
        &live,
        &mut bindings,
        &mut protected,
        &mut blocked_creates,
        &mut operations,
        &mut diagnostics,
    );
    plan_creates(
        input.capabilities(),
        &desired,
        &mut bindings,
        &blocked_creates,
        &mut operations,
        &mut diagnostics,
    );
    plan_desired_state(
        input.capabilities(),
        &desired,
        &bindings,
        &mut operations,
        &mut diagnostics,
    );
    plan_focus(
        input,
        &desired,
        &bindings,
        &mut operations,
        &mut diagnostics,
    );
    plan_closes(
        input.capabilities(),
        &desired,
        &live,
        &protected,
        &mut operations,
        &mut diagnostics,
    );

    operations.sort_by(|left, right| {
        (left.kind(), left.window_id(), left.transport_handle()).cmp(&(
            right.kind(),
            right.window_id(),
            right.transport_handle(),
        ))
    });
    diagnostics.sort();

    Ok(WindowDiffReceipt::new(
        input.generation(),
        operations
            .into_iter()
            .map(|operation| PlannedWindowOperation::new(input.generation(), operation))
            .collect(),
        diagnostics,
    ))
}

fn collect_desired(
    windows: &[DesiredWindow],
) -> Result<BTreeMap<WindowId, DesiredWindow>, WindowDiffError> {
    let mut result = BTreeMap::new();
    for window in windows {
        if result
            .insert(window.window_id().clone(), window.clone())
            .is_some()
        {
            return Err(WindowDiffError::DuplicateDesiredWindowId(
                window.window_id().clone(),
            ));
        }
    }
    Ok(result)
}

fn collect_live(
    windows: &[LiveWindow],
) -> Result<BTreeMap<HostWindowHandle, LiveWindow>, WindowDiffError> {
    let mut result = BTreeMap::new();
    let mut identities = BTreeSet::new();
    for window in windows {
        if let Some(window_id) = window.window_id() {
            if !identities.insert(window_id.clone()) {
                return Err(WindowDiffError::DuplicateLiveWindowId(window_id.clone()));
            }
        }
        if result
            .insert(window.transport_handle().clone(), window.clone())
            .is_some()
        {
            return Err(WindowDiffError::DuplicateTransportHandle(
                window.transport_handle().clone(),
            ));
        }
    }
    Ok(result)
}

fn stable_bindings(
    desired: &BTreeMap<WindowId, DesiredWindow>,
    live: &BTreeMap<HostWindowHandle, LiveWindow>,
) -> BTreeMap<WindowId, Binding> {
    live.values()
        .filter_map(|window| {
            let window_id = window.window_id()?;
            desired
                .contains_key(window_id)
                .then(|| (window_id.clone(), Binding::Existing(window.clone())))
        })
        .collect()
}

fn plan_closes(
    capabilities: &HostCapabilities,
    desired: &BTreeMap<WindowId, DesiredWindow>,
    live: &BTreeMap<HostWindowHandle, LiveWindow>,
    protected: &BTreeSet<HostWindowHandle>,
    operations: &mut Vec<WindowOperation>,
    diagnostics: &mut Vec<WindowDiffDiagnostic>,
) {
    for window in live.values() {
        if protected.contains(window.transport_handle()) {
            continue;
        }
        let Some(window_id) = window.window_id() else {
            diagnostics.push(WindowDiffDiagnostic::UnidentifiedLiveWindow {
                transport_handle: window.transport_handle().clone(),
            });
            continue;
        };
        if desired.contains_key(window_id) {
            continue;
        }
        push_supported(
            WindowOperation::Close {
                window_id: window_id.clone(),
                transport_handle: window.transport_handle().clone(),
            },
            HostCapability::Close,
            capabilities,
            operations,
            diagnostics,
        );
    }
}

pub(super) fn push_supported(
    operation: WindowOperation,
    capability: HostCapability,
    capabilities: &HostCapabilities,
    operations: &mut Vec<WindowOperation>,
    diagnostics: &mut Vec<WindowDiffDiagnostic>,
) -> bool {
    if capabilities.supports(capability) {
        operations.push(operation);
        true
    } else {
        diagnostics.push(WindowDiffDiagnostic::UnsupportedOperation {
            operation: operation.kind(),
            window_id: operation.window_id().clone(),
            transport_handle: operation.transport_handle().cloned(),
            required_capability: capability,
        });
        false
    }
}
