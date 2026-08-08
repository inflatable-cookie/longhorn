//! Captured display and placement evidence.

use longhorn_core::{DisplayId, PhysicalRect, ScaleFactor, ScreenRect, WindowId, WindowPlacement};
use longhorn_windowing::{
    ApplyGeneration, CaptureGeneration, CaptureReason, FlushReason, IgnoreReason, MonotonicMillis,
    SavedDisplayAssociation, SavedDisplayEvidence, SavedWindowPlacement, WindowLifecycleDuration,
    WindowLifecycleEvent, WindowLifecycleEventKind, resolve_saved_display_association,
};
use serde::{Deserialize, Serialize};

/// Raw current-monitor evidence without canonical display identity.
/// Raw current-monitor evidence without canonical display identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapturedDisplayEvidence {
    machine_label: Option<String>,
    full_bounds: PhysicalRect,
    work_area: PhysicalRect,
    scale: ScaleFactor,
    logical_full_bounds: ScreenRect,
    logical_work_area: ScreenRect,
}

impl CapturedDisplayEvidence {
    /// Constructs current-monitor evidence.
    #[must_use]
    pub const fn new(
        machine_label: Option<String>,
        full_bounds: PhysicalRect,
        work_area: PhysicalRect,
        scale: ScaleFactor,
        logical_full_bounds: ScreenRect,
        logical_work_area: ScreenRect,
    ) -> Self {
        Self {
            machine_label,
            full_bounds,
            work_area,
            scale,
            logical_full_bounds,
            logical_work_area,
        }
    }

    /// Returns the machine-provided label.
    #[must_use]
    pub const fn machine_label(&self) -> Option<&String> {
        self.machine_label.as_ref()
    }

    /// Returns raw physical full bounds.
    #[must_use]
    pub const fn full_bounds(&self) -> PhysicalRect {
        self.full_bounds
    }

    /// Returns raw physical work area.
    #[must_use]
    pub const fn work_area(&self) -> PhysicalRect {
        self.work_area
    }

    /// Returns validated scale evidence.
    #[must_use]
    pub const fn scale(&self) -> ScaleFactor {
        self.scale
    }

    /// Returns full bounds in the mapper's global logical plane.
    #[must_use]
    pub const fn logical_full_bounds(&self) -> ScreenRect {
        self.logical_full_bounds
    }

    /// Returns usable bounds in the mapper's global logical plane.
    #[must_use]
    pub const fn logical_work_area(&self) -> ScreenRect {
        self.logical_work_area
    }
}

/// Current-monitor observation outcome.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CapturedDisplayAssociation {
    /// Tauri returned current-monitor facts without assigning a `DisplayId`.
    Observed {
        /// Raw correlation evidence for consumer policy.
        evidence: CapturedDisplayEvidence,
    },
    /// Tauri reported no current monitor.
    Unresolved,
}

/// Complete schema-opaque settled placement proposal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapturedWindowPlacement {
    window_id: WindowId,
    normal_placement: WindowPlacement,
    maximized: bool,
    display: CapturedDisplayAssociation,
}

impl CapturedWindowPlacement {
    /// Constructs one complete placement proposal.
    #[must_use]
    pub const fn new(
        window_id: WindowId,
        normal_placement: WindowPlacement,
        maximized: bool,
        display: CapturedDisplayAssociation,
    ) -> Self {
        Self {
            window_id,
            normal_placement,
            maximized,
            display,
        }
    }

    /// Returns stable managed identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns normal outer-origin plus inner-size placement.
    #[must_use]
    pub const fn normal_placement(&self) -> WindowPlacement {
        self.normal_placement
    }

    /// Returns captured maximized state.
    #[must_use]
    pub const fn is_maximized(&self) -> bool {
        self.maximized
    }

    /// Returns observed or explicitly unresolved current-monitor evidence.
    #[must_use]
    pub const fn display(&self) -> &CapturedDisplayAssociation {
        &self.display
    }

    /// Converts this capture into the shared serializable restore record.
    #[must_use]
    pub fn saved(&self, display_id: Option<DisplayId>) -> SavedWindowPlacement {
        let evidence = match &self.display {
            CapturedDisplayAssociation::Observed { evidence } => Some(SavedDisplayEvidence::new(
                evidence.logical_full_bounds(),
                evidence.logical_work_area(),
                evidence.scale(),
            )),
            CapturedDisplayAssociation::Unresolved => None,
        };
        SavedWindowPlacement::new(
            self.window_id.clone(),
            self.normal_placement,
            self.maximized,
            SavedDisplayAssociation::new(display_id, evidence),
        )
    }

    /// Converts this capture using unique exact evidence from an inventory.
    #[must_use]
    pub fn saved_with_inventory(
        &self,
        inventory: &longhorn_display::DisplayInventory,
    ) -> SavedWindowPlacement {
        let unresolved = self.saved(None);
        let display_id = resolve_saved_display_association(unresolved.display(), inventory);
        self.saved(display_id)
    }

    /// Converts this capture using unique exact evidence from a known registry.
    ///
    /// This is useful at persistence seams that retain the registry but not the
    /// ephemeral inventory from the last restore observation.
    #[must_use]
    pub fn saved_with_registry(
        &self,
        registry: &longhorn_display::KnownDisplayRegistry,
    ) -> SavedWindowPlacement {
        let unresolved = self.saved(None);
        let Some(evidence) = unresolved.display().evidence() else {
            return unresolved;
        };
        let mut matches = registry.iter().filter(|display| {
            display.facts().full_bounds() == evidence.full_bounds()
                && display.facts().work_area() == evidence.work_area()
                && display.facts().scale() == evidence.scale()
        });
        let display_id = matches.next().map(|display| display.id().clone());
        if matches.next().is_some() {
            return unresolved;
        }
        self.saved(display_id)
    }
}

