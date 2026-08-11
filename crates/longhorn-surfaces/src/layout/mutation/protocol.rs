use longhorn_core::{
    LayoutRequestId, PanelDefinitionId, PanelInstanceId, RegionId, SizingSlotId, SurfaceId,
    SurfaceRevision,
};
use serde::{Deserialize, Serialize};

use crate::{LayoutRatio, SurfaceDocument};

/// One strict expected-revision layout mutation request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct LayoutMutationRequest {
    request_id: LayoutRequestId,
    expected_revision: SurfaceRevision,
    command: LayoutMutationCommand,
}

impl LayoutMutationRequest {
    /// Constructs one mutation request.
    #[must_use]
    pub const fn new(
        request_id: LayoutRequestId,
        expected_revision: SurfaceRevision,
        command: LayoutMutationCommand,
    ) -> Self {
        Self {
            request_id,
            expected_revision,
            command,
        }
    }

    /// Returns stable request identity.
    #[must_use]
    pub const fn request_id(&self) -> &LayoutRequestId {
        &self.request_id
    }

    /// Returns the revision required for admission.
    #[must_use]
    pub const fn expected_revision(&self) -> SurfaceRevision {
        self.expected_revision
    }

    /// Returns the requested command.
    #[must_use]
    pub const fn command(&self) -> &LayoutMutationCommand {
        &self.command
    }
}

/// Authoritative layout mutation command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
pub enum LayoutMutationCommand {
    /// Creates a caller-identified panel instance at an explicit insertion index.
    CreatePanel {
        /// New durable panel-instance identity.
        panel_instance_id: PanelInstanceId,
        /// Registered panel-definition identity.
        panel_definition_id: PanelDefinitionId,
        /// Target layout surface.
        surface_id: SurfaceId,
        /// Target semantic region.
        region_id: RegionId,
        /// Zero-based insertion index, including append at current length.
        insertion_index: u32,
    },
    /// Closes one existing closeable panel instance.
    ClosePanel {
        /// Existing durable panel-instance identity.
        panel_instance_id: PanelInstanceId,
    },
    /// Selects one existing panel instance in its current region.
    ActivatePanel {
        /// Existing durable panel-instance identity.
        panel_instance_id: PanelInstanceId,
    },
    /// Replaces one region's tab order with an exact complete permutation.
    ReorderRegion {
        /// Target layout surface.
        surface_id: SurfaceId,
        /// Target semantic region.
        region_id: RegionId,
        /// Complete ordered permutation of current region members.
        panel_instance_ids: Vec<PanelInstanceId>,
    },
    /// Moves one movable panel to a distinct region at an explicit index.
    MovePanel {
        /// Existing durable panel-instance identity.
        panel_instance_id: PanelInstanceId,
        /// Target layout surface.
        target_surface_id: SurfaceId,
        /// Target semantic region.
        target_region_id: RegionId,
        /// Zero-based insertion index in the target before insertion.
        insertion_index: u32,
    },
    /// Sets one registered sizing-slot ratio.
    SetSizingSlot {
        /// Target layout surface.
        surface_id: SurfaceId,
        /// Target sizing slot.
        sizing_slot_id: SizingSlotId,
        /// New bounded integer-millionth ratio.
        ratio: LayoutRatio,
    },
    /// Sets durable collapse state on a supported region.
    SetRegionCollapsed {
        /// Target layout surface.
        surface_id: SurfaceId,
        /// Target semantic region.
        region_id: RegionId,
        /// New collapse state.
        collapsed: bool,
    },
}

/// Command-specific committed mutation evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
pub enum LayoutMutationOutcome {
    /// One panel instance was created and activated.
    PanelCreated {
        /// Created panel instance.
        panel_instance_id: PanelInstanceId,
        /// Committed surface.
        surface_id: SurfaceId,
        /// Committed region.
        region_id: RegionId,
        /// Committed insertion index.
        insertion_index: u32,
    },
    /// One panel instance was closed.
    PanelClosed {
        /// Closed panel instance.
        panel_instance_id: PanelInstanceId,
        /// Former surface.
        surface_id: SurfaceId,
        /// Former region.
        region_id: RegionId,
        /// Former zero-based index.
        former_index: u32,
    },
    /// One panel instance became active.
    PanelActivated {
        /// Activated panel instance.
        panel_instance_id: PanelInstanceId,
        /// Containing layout surface.
        surface_id: SurfaceId,
        /// Containing semantic region.
        region_id: RegionId,
        /// Previous active member, when present.
        previous_active_panel_instance_id: Option<PanelInstanceId>,
    },
    /// One region accepted a complete committed tab order.
    RegionReordered {
        /// Reordered layout surface.
        surface_id: SurfaceId,
        /// Reordered semantic region.
        region_id: RegionId,
        /// Complete committed order.
        panel_instance_ids: Vec<PanelInstanceId>,
    },
    /// One panel moved atomically between distinct regions.
    PanelMoved {
        /// Moved panel instance.
        panel_instance_id: PanelInstanceId,
        /// Former surface.
        source_surface_id: SurfaceId,
        /// Former region.
        source_region_id: RegionId,
        /// Former zero-based index.
        former_index: u32,
        /// Committed target surface.
        target_surface_id: SurfaceId,
        /// Committed target region.
        target_region_id: RegionId,
        /// Committed target insertion index.
        insertion_index: u32,
    },
    /// One sizing slot changed.
    SizingSlotSet {
        /// Mutated layout surface.
        surface_id: SurfaceId,
        /// Mutated sizing slot.
        sizing_slot_id: SizingSlotId,
        /// Previous ratio.
        previous_ratio: LayoutRatio,
        /// Committed ratio.
        committed_ratio: LayoutRatio,
    },
    /// One region's collapse state changed.
    RegionCollapsedSet {
        /// Mutated layout surface.
        surface_id: SurfaceId,
        /// Mutated semantic region.
        region_id: RegionId,
        /// Previous supported collapse state.
        previous_collapsed: bool,
        /// Committed collapse state.
        committed_collapsed: bool,
    },
}

/// Successful authoritative layout mutation receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct LayoutMutationReceipt {
    request_id: LayoutRequestId,
    previous_revision: SurfaceRevision,
    committed_revision: SurfaceRevision,
    outcome: LayoutMutationOutcome,
    authoritative_document: SurfaceDocument,
}

impl LayoutMutationReceipt {
    pub(super) fn new(
        request_id: LayoutRequestId,
        previous_revision: SurfaceRevision,
        committed_revision: SurfaceRevision,
        outcome: LayoutMutationOutcome,
        authoritative_document: SurfaceDocument,
    ) -> Self {
        Self {
            request_id,
            previous_revision,
            committed_revision,
            outcome,
            authoritative_document,
        }
    }

    /// Returns stable request identity.
    #[must_use]
    pub const fn request_id(&self) -> &LayoutRequestId {
        &self.request_id
    }

    /// Returns the admitted source revision.
    #[must_use]
    pub const fn previous_revision(&self) -> SurfaceRevision {
        self.previous_revision
    }

    /// Returns the single committed successor revision.
    #[must_use]
    pub const fn committed_revision(&self) -> SurfaceRevision {
        self.committed_revision
    }

    /// Returns command-specific committed evidence.
    #[must_use]
    pub const fn outcome(&self) -> &LayoutMutationOutcome {
        &self.outcome
    }

    /// Returns the complete normalized authoritative document.
    #[must_use]
    pub const fn authoritative_document(&self) -> &SurfaceDocument {
        &self.authoritative_document
    }
}
