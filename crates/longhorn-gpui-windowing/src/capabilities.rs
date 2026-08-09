use longhorn_windowing::{
    DeferredSettlement, HostCapabilities, HostCapability, WindowOperationKind,
};

/// Why a GPUI host withholds a capability Tauri declares.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WithheldCapability {
    /// The capability a GPUI host cannot declare.
    pub capability: HostCapability,
    /// The GPUI-side reason, stated without reference to Tauri.
    pub reason: &'static str,
}

/// Every capability a GPUI host must withhold, with its cause.
///
/// The pure planner turns a withheld capability into a
/// `WindowDiffDiagnostic::UnsupportedOperation`, so a GPUI application gets a
/// named refusal for each window it cannot place rather than a silent
/// mis-apply. That mechanism is host-neutral and needed no change; what the
/// list below records is which requirements of contract 020 a GPUI host
/// cannot meet.
pub const WITHHELD_CAPABILITIES: [WithheldCapability; 3] = [
    WithheldCapability {
        capability: HostCapability::Move,
        reason: "gpui's PlatformWindow has no position setter; bounds are a \
                 creation-time option and cannot be changed afterwards",
    },
    WithheldCapability {
        capability: HostCapability::Show,
        reason: "gpui windows are on screen from creation; there is no runtime show",
    },
    WithheldCapability {
        capability: HostCapability::Hide,
        reason: "gpui has no runtime hide; the nearest operation, minimize, is a \
                 different state a user can reverse",
    },
];

/// Derives the exact capability set a GPUI host can honestly declare.
///
/// A host declares only what it can do. `Move`, `Show` and `Hide` are absent
/// for the reasons in [`WITHHELD_CAPABILITIES`]; declaring them would make the
/// planner emit operations the adapter would have to fail or fake.
///
/// `Resize` is present. It was not, while the capability vocabulary carried a
/// compound `MoveResize`: GPUI has `Window::resize` and no position setter, so
/// declaring the compound would have been a lie and withholding it left a GPUI
/// window unresizable from a plan. Splitting the capability is what made the
/// honest answer expressible.
#[must_use]
pub fn gpui_host_capabilities(can_create: bool) -> HostCapabilities {
    let mut capabilities = vec![
        HostCapability::Retag,
        HostCapability::Resize,
        HostCapability::Maximize,
        HostCapability::Unmaximize,
        HostCapability::Focus,
        HostCapability::Close,
    ];
    if can_create {
        capabilities.push(HostCapability::Create);
    }
    HostCapabilities::from_capabilities(capabilities)
}

/// The operations a GPUI host cannot observe settling in the same turn.
///
/// Measured, not reasoned about. `prototypes/gpui-windowing`'s smoke binary
/// called `set_maximized(true)` against a real window on macOS 25.5, got
/// success, and the next `is_maximized()` still reported `false` — the window
/// server animates the zoom. A convergence diff taken immediately after apply
/// therefore reports an unconverged maximize that in fact succeeded, and a
/// caller that trusted it would reschedule the operation forever.
///
/// Move and resize are not here: the same smoke run observed a created window
/// at exactly the origin the plan asked for.
#[must_use]
pub fn gpui_deferred_settlement() -> DeferredSettlement {
    DeferredSettlement::from_operations([
        WindowOperationKind::Maximize,
        WindowOperationKind::Unmaximize,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gpui_host_declares_only_what_gpui_can_do() {
        let capabilities = gpui_host_capabilities(true);

        for supported in [
            HostCapability::Create,
            HostCapability::Retag,
            HostCapability::Resize,
            HostCapability::Maximize,
            HostCapability::Unmaximize,
            HostCapability::Focus,
            HostCapability::Close,
        ] {
            assert!(capabilities.supports(supported), "{supported:?}");
        }
        for withheld in WITHHELD_CAPABILITIES {
            assert!(
                !capabilities.supports(withheld.capability),
                "{:?}",
                withheld.capability
            );
        }
    }

    #[test]
    fn a_host_with_no_factory_cannot_create() {
        assert!(!gpui_host_capabilities(false).supports(HostCapability::Create));
    }
}
