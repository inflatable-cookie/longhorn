use longhorn_core::{OperationCatalogueRevision, OperationId, OperationRevision};

use crate::{
    OperationAuthorityCursor, OperationCatalogueLimits, OperationRecord, OperationSequence,
};

/// Why a retained terminal record left the catalogue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationRemovalReason {
    /// Finite terminal retention removed the oldest eligible record.
    Evicted,
    /// A caller explicitly dismissed one terminal record.
    Dismissed,
}

/// Exact terminal metadata removed from the catalogue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationRemoval {
    operation_id: OperationId,
    sequence: OperationSequence,
    encoded_weight: u64,
    reason: OperationRemovalReason,
}

impl OperationRemoval {
    pub(crate) fn from_record(record: &OperationRecord, reason: OperationRemovalReason) -> Self {
        Self {
            operation_id: record.operation_id().clone(),
            sequence: record.sequence(),
            encoded_weight: record.encoded_metadata_weight(),
            reason,
        }
    }

    /// Returns the removed operation identity.
    #[must_use]
    pub const fn operation_id(&self) -> &OperationId {
        &self.operation_id
    }

    /// Returns the insertion sequence of the removed operation.
    #[must_use]
    pub const fn sequence(&self) -> OperationSequence {
        self.sequence
    }

    /// Returns the canonical metadata weight removed.
    #[must_use]
    pub const fn encoded_weight(&self) -> u64 {
        self.encoded_weight
    }

    /// Returns why the record was removed.
    #[must_use]
    pub const fn reason(&self) -> OperationRemovalReason {
        self.reason
    }
}

/// Revision-bound change to finite catalogue limits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationRetentionChange {
    pub(crate) authority: OperationAuthorityCursor,
    pub(crate) expected_catalogue_revision: OperationCatalogueRevision,
    pub(crate) limits: OperationCatalogueLimits,
}

impl OperationRetentionChange {
    /// Constructs a retention-limit change.
    #[must_use]
    pub const fn new(
        authority: OperationAuthorityCursor,
        expected_catalogue_revision: OperationCatalogueRevision,
        limits: OperationCatalogueLimits,
    ) -> Self {
        Self {
            authority,
            expected_catalogue_revision,
            limits,
        }
    }
}

/// Receipt for a finite retention-limit change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationRetentionReceipt {
    pub(crate) previous_limits: OperationCatalogueLimits,
    pub(crate) committed_limits: OperationCatalogueLimits,
    pub(crate) previous_catalogue_revision: OperationCatalogueRevision,
    pub(crate) committed_catalogue_revision: OperationCatalogueRevision,
    pub(crate) evicted: Vec<OperationRemoval>,
    pub(crate) retained_terminal_encoded_weight: u64,
}

impl OperationRetentionReceipt {
    /// Returns the replaced limits.
    #[must_use]
    pub const fn previous_limits(&self) -> OperationCatalogueLimits {
        self.previous_limits
    }

    /// Returns the authoritative limits.
    #[must_use]
    pub const fn committed_limits(&self) -> OperationCatalogueLimits {
        self.committed_limits
    }

    /// Returns the catalogue revision before the change.
    #[must_use]
    pub const fn previous_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.previous_catalogue_revision
    }

    /// Returns the catalogue revision after the change.
    #[must_use]
    pub const fn committed_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.committed_catalogue_revision
    }

    /// Returns oldest-first terminal eviction evidence.
    #[must_use]
    pub fn evicted(&self) -> &[OperationRemoval] {
        &self.evicted
    }

    /// Returns retained terminal metadata weight.
    #[must_use]
    pub const fn retained_terminal_encoded_weight(&self) -> u64 {
        self.retained_terminal_encoded_weight
    }
}

/// Revision-bound explicit dismissal of one terminal projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationDismissal {
    pub(crate) authority: OperationAuthorityCursor,
    pub(crate) operation_id: OperationId,
    pub(crate) expected_operation_revision: OperationRevision,
}

impl OperationDismissal {
    /// Constructs an explicit terminal dismissal.
    #[must_use]
    pub const fn new(
        authority: OperationAuthorityCursor,
        operation_id: OperationId,
        expected_operation_revision: OperationRevision,
    ) -> Self {
        Self {
            authority,
            operation_id,
            expected_operation_revision,
        }
    }
}

/// Receipt for one explicit terminal dismissal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationDismissalReceipt {
    pub(crate) removed: OperationRemoval,
    pub(crate) previous_catalogue_revision: OperationCatalogueRevision,
    pub(crate) committed_catalogue_revision: OperationCatalogueRevision,
}

impl OperationDismissalReceipt {
    /// Returns exact removed metadata.
    #[must_use]
    pub const fn removed(&self) -> &OperationRemoval {
        &self.removed
    }

    /// Returns the catalogue revision before dismissal.
    #[must_use]
    pub const fn previous_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.previous_catalogue_revision
    }

    /// Returns the committed catalogue revision.
    #[must_use]
    pub const fn committed_catalogue_revision(&self) -> OperationCatalogueRevision {
        self.committed_catalogue_revision
    }
}
