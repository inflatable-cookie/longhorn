use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::WindowId;

use super::{Binding, push_supported};
use crate::diff::{
    DesiredWindow, HostCapability, HostWindowHandle, LiveWindow, ProtectedPrimaryPolicy,
    WindowDiffDiagnostic, WindowDiffInput, WindowOperation,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn plan_protected_primary(
    input: &WindowDiffInput,
    desired: &BTreeMap<WindowId, DesiredWindow>,
    live: &BTreeMap<HostWindowHandle, LiveWindow>,
    bindings: &mut BTreeMap<WindowId, Binding>,
    protected: &mut BTreeSet<HostWindowHandle>,
    blocked_creates: &mut BTreeSet<WindowId>,
    operations: &mut Vec<WindowOperation>,
    diagnostics: &mut Vec<WindowDiffDiagnostic>,
) {
    let (handle, target) = match input.protected_primary() {
        ProtectedPrimaryPolicy::None => return,
        ProtectedPrimaryPolicy::Preserve { transport_handle } => (transport_handle, None),
        ProtectedPrimaryPolicy::Reuse {
            transport_handle,
            window_id,
        } => (transport_handle, Some(window_id)),
    };
    protected.insert(handle.clone());
    let Some(slot) = live.get(handle) else {
        diagnostics.push(WindowDiffDiagnostic::ProtectedSlotMissing {
            transport_handle: handle.clone(),
        });
        if let Some(target) = target {
            blocked_creates.insert(target.clone());
        }
        return;
    };
    let Some(target) = target else {
        return;
    };
    if !desired.contains_key(target) {
        diagnostics.push(WindowDiffDiagnostic::ProtectedReuseTargetMissing {
            window_id: target.clone(),
        });
        return;
    }
    if slot.window_id() == Some(target) {
        return;
    }
    if let Some(Binding::Existing(matched)) = bindings.get(target) {
        diagnostics.push(WindowDiffDiagnostic::ProtectedReuseConflict {
            protected_handle: handle.clone(),
            window_id: target.clone(),
            matched_handle: matched.transport_handle().clone(),
        });
        return;
    }

    let operation = WindowOperation::Retag {
        window_id: target.clone(),
        transport_handle: handle.clone(),
    };
    if push_supported(
        operation,
        HostCapability::Retag,
        input.capabilities(),
        operations,
        diagnostics,
    ) {
        if let Some(old_id) = slot.window_id() {
            bindings.remove(old_id);
        }
        bindings.insert(target.clone(), Binding::Existing(slot.clone()));
    } else {
        blocked_creates.insert(target.clone());
    }
}
