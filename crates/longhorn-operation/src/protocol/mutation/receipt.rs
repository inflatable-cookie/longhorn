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
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum OperationMutationReceiptProjection {
    /// A new operation was registered.
    Registered {
        /// Committed operation entry.
        operation: OperationEntryProjection,
        /// Catalogue revision before the mutation.
        previous_catalogue_revision: OperationCatalogueRevision,
        /// Catalogue revision after the mutation.
        committed_catalogue_revision: OperationCatalogueRevision,
    },
    /// Progress was reported for an operation.
    Progressed {
        /// Progressed operation identity.
        operation_id: OperationId,
        /// Operation revision before the mutation.
        previous_operation_revision: OperationRevision,
        /// Operation revision after the mutation.
        committed_operation_revision: OperationRevision,
        /// Progress sequence before the mutation.
        #[cfg_attr(feature = "bindings", ts(type = "number"))]
        previous_progress_sequence: u64,
        /// Committed progress state.
        committed_progress: OperationProgressProjection,
        /// Catalogue revision before the mutation.
        previous_catalogue_revision: OperationCatalogueRevision,
        /// Catalogue revision after the mutation.
        committed_catalogue_revision: OperationCatalogueRevision,
    },
    /// An operation transitioned state.
    Transitioned {
        /// Transitioned operation identity.
        operation_id: OperationId,
        /// State before the mutation.
        previous_state: OperationStateProjection,
        /// State after the mutation.
        committed_state: OperationStateProjection,
        /// Operation revision before the mutation.
        previous_operation_revision: OperationRevision,
        /// Operation revision after the mutation.
        committed_operation_revision: OperationRevision,
        /// Catalogue revision before the mutation.
        previous_catalogue_revision: OperationCatalogueRevision,
        /// Catalogue revision after the mutation.
        committed_catalogue_revision: OperationCatalogueRevision,
        /// Operations the transition evicted.
        evicted: Vec<OperationRemovalProjection>,
    },
    /// Retention limits changed.
    RetentionChanged {
        /// Retention limits before the mutation.
        previous_limits: OperationCatalogueLimitsProjection,
        /// Retention limits after the mutation.
        committed_limits: OperationCatalogueLimitsProjection,
        /// Catalogue revision before the mutation.
        previous_catalogue_revision: OperationCatalogueRevision,
        /// Catalogue revision after the mutation.
        committed_catalogue_revision: OperationCatalogueRevision,
        /// Operations the new limits evicted.
        evicted: Vec<OperationRemovalProjection>,
        /// Encoded weight retained after eviction.
        #[cfg_attr(feature = "bindings", ts(type = "number"))]
        retained_terminal_encoded_weight: u64,
    },
    /// An operation was dismissed.
    Dismissed {
        /// Removed operation record.
        removed: OperationRemovalProjection,
        /// Catalogue revision before the mutation.
        previous_catalogue_revision: OperationCatalogueRevision,
        /// Catalogue revision after the mutation.
        committed_catalogue_revision: OperationCatalogueRevision,
    },
    /// The catalogue was torn down.
    TornDown {
        /// Catalogue revision before the mutation.
        previous_catalogue_revision: OperationCatalogueRevision,
        /// Catalogue revision after the mutation.
        committed_catalogue_revision: OperationCatalogueRevision,
        /// Per-operation teardown results.
        outcomes: Vec<OperationTeardownOutcomeProjection>,
        /// Operations the teardown evicted.
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum OperationTeardownOutcomeProjection {
    /// The operation resolved to a terminal state.
    Completed {
        /// Torn-down operation identity.
        operation_id: OperationId,
        /// Terminal state the operation reached.
        state: OperationStateProjection,
        /// Operation revision before teardown.
        previous_operation_revision: OperationRevision,
        /// Operation revision after teardown.
        committed_operation_revision: OperationRevision,
    },
    /// The operation transferred to another authority.
    Transferred {
        /// Transferred operation identity.
        operation_id: OperationId,
        /// Operation revision before teardown.
        previous_operation_revision: OperationRevision,
        /// Authority the operation transferred to.
        target_authority: OperationAuthorityProjection,
    },
}
