use longhorn_core::{OperationCatalogueRevision, OperationId, OperationRevision};

use crate::{OperationAuthorityCursor, OperationRemoval, OperationState};

/// One explicit active-operation outcome supplied during controlled teardown.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationTeardownResolutionOutcome {
    /// Commit a consumer-proven terminal fact.
    Complete(OperationState),
    /// Remove the record from this authority and name its new live authority.
    Transfer(OperationAuthorityCursor),
}

/// Revision-bound resolution of one active operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationTeardownResolution {
    pub(crate) operation_id: OperationId,
    pub(crate) expected_operation_revision: OperationRevision,
    pub(crate) outcome: OperationTeardownResolutionOutcome,
}

impl OperationTeardownResolution {
    /// Constructs one teardown resolution.
    #[must_use]
    pub const fn new(
        operation_id: OperationId,
        expected_operation_revision: OperationRevision,
        outcome: OperationTeardownResolutionOutcome,
    ) -> Self {
        Self {
            operation_id,
            expected_operation_revision,
            outcome,
        }
    }
}

/// Complete controlled teardown command for one authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationTeardown {
    pub(crate) authority: OperationAuthorityCursor,
    pub(crate) expected_catalogue_revision: OperationCatalogueRevision,
    pub(crate) resolutions: Vec<OperationTeardownResolution>,
}

impl OperationTeardown {
    /// Constructs a teardown command. Every active operation must appear exactly once.
    #[must_use]
    pub const fn new(
        authority: OperationAuthorityCursor,
        expected_catalogue_revision: OperationCatalogueRevision,
        resolutions: Vec<OperationTeardownResolution>,
    ) -> Self {
        Self {
            authority,
            expected_catalogue_revision,
            resolutions,
        }
    }
}

/// Exact result for one active operation during teardown.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationTeardownOutcome {
    /// This authority committed a terminal fact.
    Completed {
        /// Resolved operation identity.
        operation_id: OperationId,
        /// Consumer-proven terminal fact.
        state: OperationState,
        /// Revision before teardown.
        previous_operation_revision: OperationRevision,
        /// Terminal revision committed by teardown.
        committed_operation_revision: OperationRevision,
    },
    /// This authority removed the record and named the receiving authority.
    Transferred {
        /// Transferred operation identity.
        operation_id: OperationId,
        /// Last revision owned by the closing authority.
        previous_operation_revision: OperationRevision,
        /// Consumer-supplied receiving live authority.
        target_authority: OperationAuthorityCursor,
    },
}

impl OperationTeardownOutcome {
    /// Returns the resolved operation identity.
    #[must_use]
    pub const fn operation_id(&self) -> &OperationId {
        match self {
            Self::Completed { operation_id, .. } | Self::Transferred { operation_id, .. } => {
                operation_id
            }
        }
    }
}

/// Receipt proving complete, atomic controlled teardown.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationTeardownReceipt {
    pub(crate) previous_catalogue_revision: OperationCatalogueRevision,
    pub(crate) committed_catalogue_revision: OperationCatalogueRevision,
    pub(crate) outcomes: Vec<OperationTeardownOutcome>,
    pub(crate) evicted: Vec<OperationRemoval>,
}

impl OperationTeardownReceipt {
    /// Returns the catalogue revision before teardown.
    #[must_use]
    pub const fn previous_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.previous_catalogue_revision
    }
    /// Returns the final catalogue revision.
    #[must_use]
    pub const fn committed_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.committed_catalogue_revision
    }
    /// Returns one exact outcome per formerly active operation.
    #[must_use]
    pub fn outcomes(&self) -> &[OperationTeardownOutcome] {
        &self.outcomes
    }
    /// Returns terminal records evicted while committing teardown.
    #[must_use]
    pub fn evicted(&self) -> &[OperationRemoval] {
        &self.evicted
    }
}
