use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::WindowId;

use super::{Binding, push_supported};
use crate::diff::{
    DesiredWindow, FocusPolicy, HostCapabilities, HostCapability, HostWindowHandle, LiveWindow,
    WindowDiffDiagnostic, WindowDiffInput, WindowOperation,
};

pub(super) fn plan_creates(
    capabilities: &HostCapabilities,
    desired: &BTreeMap<WindowId, DesiredWindow>,
    bindings: &mut BTreeMap<WindowId, Binding>,
    blocked: &BTreeSet<WindowId>,
    operations: &mut Vec<WindowOperation>,
    diagnostics: &mut Vec<WindowDiffDiagnostic>,
) {
    for window_id in desired.keys() {
        if bindings.contains_key(window_id) || blocked.contains(window_id) {
            continue;
        }
        let operation = WindowOperation::Create {
            window_id: window_id.clone(),
        };
        if push_supported(
            operation,
            HostCapability::Create,
            capabilities,
            operations,
            diagnostics,
        ) {
            bindings.insert(window_id.clone(), Binding::Created);
        }
    }
}

pub(super) fn plan_desired_state(
    capabilities: &HostCapabilities,
    desired: &BTreeMap<WindowId, DesiredWindow>,
    bindings: &BTreeMap<WindowId, Binding>,
    operations: &mut Vec<WindowOperation>,
    diagnostics: &mut Vec<WindowDiffDiagnostic>,
) {
    for (window_id, desired) in desired {
        let Some(binding) = bindings.get(window_id) else {
            continue;
        };
        match binding {
            Binding::Created => {
                push_supported(
                    move_resize(desired, None),
                    HostCapability::MoveResize,
                    capabilities,
                    operations,
                    diagnostics,
                );
                if desired.is_maximized() {
                    push_supported(
                        maximize(window_id, None),
                        HostCapability::Maximize,
                        capabilities,
                        operations,
                        diagnostics,
                    );
                }
                if desired.is_visible() {
                    push_supported(
                        show(window_id, None),
                        HostCapability::Show,
                        capabilities,
                        operations,
                        diagnostics,
                    );
                }
            }
            Binding::Existing(live) => {
                plan_existing_state(capabilities, desired, live, operations, diagnostics);
            }
        }
    }
}

fn plan_existing_state(
    capabilities: &HostCapabilities,
    desired: &DesiredWindow,
    live: &LiveWindow,
    operations: &mut Vec<WindowOperation>,
    diagnostics: &mut Vec<WindowDiffDiagnostic>,
) {
    let window_id = desired.window_id();
    let handle = Some(live.transport_handle().clone());
    if live.is_maximized() && !desired.is_maximized() {
        let unmaximized = push_supported(
            WindowOperation::Unmaximize {
                window_id: window_id.clone(),
                transport_handle: handle.clone(),
            },
            HostCapability::Unmaximize,
            capabilities,
            operations,
            diagnostics,
        );
        if unmaximized {
            push_supported(
                move_resize(desired, handle.clone()),
                HostCapability::MoveResize,
                capabilities,
                operations,
                diagnostics,
            );
        }
    } else if !live.is_maximized() {
        let metrics = live.metrics();
        let placement = desired.placement();
        if metrics.outer_bounds().origin() != placement.outer_origin()
            || metrics.inner_size() != placement.inner_size()
        {
            push_supported(
                move_resize(desired, handle.clone()),
                HostCapability::MoveResize,
                capabilities,
                operations,
                diagnostics,
            );
        }
        if desired.is_maximized() {
            push_supported(
                maximize(window_id, handle.clone()),
                HostCapability::Maximize,
                capabilities,
                operations,
                diagnostics,
            );
        }
    }

    if desired.is_visible() != live.is_visible() {
        let (operation, capability) = if desired.is_visible() {
            (show(window_id, handle), HostCapability::Show)
        } else {
            (
                WindowOperation::Hide {
                    window_id: window_id.clone(),
                    transport_handle: handle,
                },
                HostCapability::Hide,
            )
        };
        push_supported(operation, capability, capabilities, operations, diagnostics);
    }
}

pub(super) fn plan_focus(
    input: &WindowDiffInput,
    desired: &BTreeMap<WindowId, DesiredWindow>,
    bindings: &BTreeMap<WindowId, Binding>,
    operations: &mut Vec<WindowOperation>,
    diagnostics: &mut Vec<WindowDiffDiagnostic>,
) {
    let FocusPolicy::Focus(window_id) = input.focus_policy() else {
        return;
    };
    let Some(target) = desired.get(window_id) else {
        diagnostics.push(WindowDiffDiagnostic::FocusTargetMissing {
            window_id: window_id.clone(),
        });
        return;
    };
    if !target.is_visible() {
        diagnostics.push(WindowDiffDiagnostic::FocusTargetHidden {
            window_id: window_id.clone(),
        });
        return;
    }
    let Some(binding) = bindings.get(window_id) else {
        return;
    };
    if matches!(binding, Binding::Existing(live) if live.is_focused()) {
        return;
    }
    push_supported(
        WindowOperation::Focus {
            window_id: window_id.clone(),
            transport_handle: binding.handle(),
        },
        HostCapability::Focus,
        input.capabilities(),
        operations,
        diagnostics,
    );
}

fn move_resize(desired: &DesiredWindow, handle: Option<HostWindowHandle>) -> WindowOperation {
    WindowOperation::MoveResize {
        window_id: desired.window_id().clone(),
        transport_handle: handle,
        placement: desired.placement(),
    }
}

fn maximize(window_id: &WindowId, handle: Option<HostWindowHandle>) -> WindowOperation {
    WindowOperation::Maximize {
        window_id: window_id.clone(),
        transport_handle: handle,
    }
}

fn show(window_id: &WindowId, handle: Option<HostWindowHandle>) -> WindowOperation {
    WindowOperation::Show {
        window_id: window_id.clone(),
        transport_handle: handle,
    }
}
