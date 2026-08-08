//! Mutation and teardown receipt projections.

use longhorn_core::{OperationCatalogueRevision, OperationId, OperationRevision};
use serde::{Deserialize, Serialize};

use crate::protocol::{
    OperationAuthorityProjection, OperationCatalogueLimitsProjection, OperationEntryProjection,
    OperationProgressProjection, OperationStateProjection,
};
use crate::{OperationRemoval, OperationRemovalReason};

/// Why one retained terminal entry was removed.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum OperationRemovalReasonProjection {
    /// Removed by finite retention.
    Evicted,
    /// Removed by explicit dismissal.
    Dismissed,
}

/// Exact removed terminal metadata.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationRemovalProjection {
    /// Removed operation identity.
    pub operation_id: OperationId,
    /// Original insertion sequence.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub sequence: u64,
    /// Canonical removed metadata weight.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub encoded_weight: u64,
    /// Removal reason.
    pub reason: OperationRemovalReasonProjection,
}

impl From<&OperationRemoval> for OperationRemovalProjection {
    fn from(value: &OperationRemoval) -> Self {
        Self {
            operation_id: value.operation_id().clone(),
            sequence: value.sequence().get(),
            encoded_weight: value.encoded_weight(),
            reason: match value.reason() {
                OperationRemovalReason::Evicted => OperationRemovalReasonProjection::Evicted,
                OperationRemovalReason::Dismissed => OperationRemovalReasonProjection::Dismissed,
            },
        }
    }
}

/// Exact successful management receipt.
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum OperationMutationReceiptProjection {
    Registered {
        operation: OperationEntryProjection,
        previous_catalogue_revision: OperationCatalogueRevision,
        committed_catalogue_revision: OperationCatalogueRevision,
    },
    Progressed {
        operation_id: OperationId,
        previous_operation_revision: OperationRevision,
        committed_operation_revision: OperationRevision,
        #[cfg_attr(feature = "bindings", ts(type = "number"))]
        previous_progress_sequence: u64,
        committed_progress: OperationProgressProjection,
        previous_catalogue_revision: OperationCatalogueRevision,
        committed_catalogue_revision: OperationCatalogueRevision,
    },
    Transitioned {
        operation_id: OperationId,
        previous_state: OperationStateProjection,
        committed_state: OperationStateProjection,
        previous_operation_revision: OperationRevision,
        committed_operation_revision: OperationRevision,
        previous_catalogue_revision: OperationCatalogueRevision,
        committed_catalogue_revision: OperationCatalogueRevision,
        evicted: Vec<OperationRemovalProjection>,
    },
    RetentionChanged {
        previous_limits: OperationCatalogueLimitsProjection,
        committed_limits: OperationCatalogueLimitsProjection,
        previous_catalogue_revision: OperationCatalogueRevision,
        committed_catalogue_revision: OperationCatalogueRevision,
        evicted: Vec<OperationRemovalProjection>,
        #[cfg_attr(feature = "bindings", ts(type = "number"))]
        retained_terminal_encoded_weight: u64,
    },
    Dismissed {
        removed: OperationRemovalProjection,
        previous_catalogue_revision: OperationCatalogueRevision,
        committed_catalogue_revision: OperationCatalogueRevision,
    },
    TornDown {
        previous_catalogue_revision: OperationCatalogueRevision,
        committed_catalogue_revision: OperationCatalogueRevision,
        outcomes: Vec<OperationTeardownOutcomeProjection>,
        evicted: Vec<OperationRemovalProjection>,
    },
}

impl OperationMutationReceiptProjection {
    /// Returns the previous catalogue revision.
    #[must_use]
    pub const fn previous_catalogue_revision(&self) -> OperationCatalogueRevision {
        match self {
            Self::Registered {
                previous_catalogue_revision,
                ..
            }
            | Self::Progressed {
                previous_catalogue_revision,
                ..
            }
            | Self::Transitioned {
                previous_catalogue_revision,
                ..
            }
            | Self::RetentionChanged {
                previous_catalogue_revision,
                ..
            }
            | Self::Dismissed {
                previous_catalogue_revision,
                ..
            }
            | Self::TornDown {
                previous_catalogue_revision,
                ..
            } => *previous_catalogue_revision,
        }
    }

    /// Returns the committed catalogue revision.
    #[must_use]
    pub const fn committed_catalogue_revision(&self) -> OperationCatalogueRevision {
        match self {
            Self::Registered {
                committed_catalogue_revision,
                ..
            }
            | Self::Progressed {
                committed_catalogue_revision,
                ..
            }
            | Self::Transitioned {
                committed_catalogue_revision,
                ..
            }
            | Self::RetentionChanged {
                committed_catalogue_revision,
                ..
            }
            | Self::Dismissed {
                committed_catalogue_revision,
                ..
            }
            | Self::TornDown {
                committed_catalogue_revision,
                ..
            } => *committed_catalogue_revision,
        }
    }

    /// Returns one directly targeted operation when present.
    #[must_use]
    pub const fn operation_id(&self) -> Option<&OperationId> {
        match self {
            Self::Registered { operation, .. } => Some(&operation.operation_id),
            Self::Progressed { operation_id, .. } | Self::Transitioned { operation_id, .. } => {
                Some(operation_id)
            }
            Self::Dismissed { removed, .. } => Some(&removed.operation_id),
            Self::RetentionChanged { .. } | Self::TornDown { .. } => None,
        }
    }
}

/// Exact teardown result on the wire.
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum OperationTeardownOutcomeProjection {
    Completed {
        operation_id: OperationId,
        state: OperationStateProjection,
        previous_operation_revision: OperationRevision,
        committed_operation_revision: OperationRevision,
    },
    Transferred {
        operation_id: OperationId,
        previous_operation_revision: OperationRevision,
        target_authority: OperationAuthorityProjection,
    },
}
