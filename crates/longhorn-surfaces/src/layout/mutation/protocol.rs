use longhorn_core::{
    LayoutContainerId, LayoutRequestId, LayoutRevision, PanelDefinitionId, PanelInstanceId,
    RegionId, SizingSlotId,
};
use serde::{Deserialize, Serialize};

use crate::{LayoutDocument, LayoutRatio};

/// One strict expected-revision layout mutation request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct LayoutMutationRequest {
    request_id: LayoutRequestId,
    expected_revision: LayoutRevision,
    command: LayoutMutationCommand,
}

impl LayoutMutationRequest {
    /// Constructs one mutation request.
    #[must_use]
    pub const fn new(
        request_id: LayoutRequestId,
        expected_revision: LayoutRevision,
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
    pub const fn expected_revision(&self) -> LayoutRevision {
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
        /// Target layout container.
        container_id: LayoutContainerId,
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
        /// Target layout container.
        container_id: LayoutContainerId,
        /// Target semantic region.
        region_id: RegionId,
        /// Complete ordered permutation of current region members.
        panel_instance_ids: Vec<PanelInstanceId>,
    },
    /// Moves one movable panel to a distinct region at an explicit index.
    MovePanel {
        /// Existing durable panel-instance identity.
        panel_instance_id: PanelInstanceId,
        /// Target layout container.
        target_container_id: LayoutContainerId,
        /// Target semantic region.
        target_region_id: RegionId,
        /// Zero-based insertion index in the target before insertion.
        insertion_index: u32,
    },
    /// Sets one registered sizing-slot ratio.
    SetSizingSlot {
        /// Target layout container.
        container_id: LayoutContainerId,
        /// Target sizing slot.
        sizing_slot_id: SizingSlotId,
        /// New bounded integer-millionth ratio.
        ratio: LayoutRatio,
    },
    /// Sets durable collapse state on a supported region.
    SetRegionCollapsed {
        /// Target layout container.
        container_id: LayoutContainerId,
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
        /// Committed container.
        container_id: LayoutContainerId,
        /// Committed region.
        region_id: RegionId,
        /// Committed insertion index.
        insertion_index: u32,
    },
    /// One panel instance was closed.
    PanelClosed {
        /// Closed panel instance.
        panel_instance_id: PanelInstanceId,
        /// Former container.
        container_id: LayoutContainerId,
        /// Former region.
        region_id: RegionId,
        /// Former zero-based index.
        former_index: u32,
    },
    /// One panel instance became active.
    PanelActivated {
        /// Activated panel instance.
        panel_instance_id: PanelInstanceId,
        /// Containing layout container.
        container_id: LayoutContainerId,
        /// Containing semantic region.
        region_id: RegionId,
        /// Previous active member, when present.
        previous_active_panel_instance_id: Option<PanelInstanceId>,
    },
    /// One region accepted a complete committed tab order.
    RegionReordered {
        /// Reordered layout container.
        container_id: LayoutContainerId,
        /// Reordered semantic region.
        region_id: RegionId,
        /// Complete committed order.
        panel_instance_ids: Vec<PanelInstanceId>,
    },
    /// One panel moved atomically between distinct regions.
    PanelMoved {
        /// Moved panel instance.
        panel_instance_id: PanelInstanceId,
        /// Former container.
        source_container_id: LayoutContainerId,
        /// Former region.
        source_region_id: RegionId,
        /// Former zero-based index.
        former_index: u32,
        /// Committed target container.
        target_container_id: LayoutContainerId,
        /// Committed target region.
        target_region_id: RegionId,
        /// Committed target insertion index.
        insertion_index: u32,
    },
    /// One sizing slot changed.
    SizingSlotSet {
        /// Mutated layout container.
        container_id: LayoutContainerId,
        /// Mutated sizing slot.
        sizing_slot_id: SizingSlotId,
        /// Previous ratio.
        previous_ratio: LayoutRatio,
        /// Committed ratio.
        committed_ratio: LayoutRatio,
    },
    /// One region's collapse state changed.
    RegionCollapsedSet {
        /// Mutated layout container.
        container_id: LayoutContainerId,
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
    previous_revision: LayoutRevision,
    committed_revision: LayoutRevision,
    outcome: LayoutMutationOutcome,
    authoritative_document: LayoutDocument,
}

impl LayoutMutationReceipt {
    pub(super) fn new(
        request_id: LayoutRequestId,
        previous_revision: LayoutRevision,
        committed_revision: LayoutRevision,
        outcome: LayoutMutationOutcome,
        authoritative_document: LayoutDocument,
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
    pub const fn previous_revision(&self) -> LayoutRevision {
        self.previous_revision
    }

    /// Returns the single committed successor revision.
    #[must_use]
    pub const fn committed_revision(&self) -> LayoutRevision {
        self.committed_revision
    }

    /// Returns command-specific committed evidence.
    #[must_use]
    pub const fn outcome(&self) -> &LayoutMutationOutcome {
        &self.outcome
    }

    /// Returns the complete normalized authoritative document.
    #[must_use]
    pub const fn authoritative_document(&self) -> &LayoutDocument {
        &self.authoritative_document
    }
}
