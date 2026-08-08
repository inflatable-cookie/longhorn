//! Validated mutable authority for one finite operation catalogue.

use longhorn_core::{OperationAuthorityId, OperationCatalogueRevision, OperationId};

use crate::{
    OperationAuthorityCursor, OperationAuthorityEpoch, OperationCatalogueError,
    OperationCatalogueLimits, OperationRecord, OperationSequence,
};

use super::terminal_weight;

/// Validated mutable authority for one finite operation catalogue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationCatalogue {
    pub(crate) authority: OperationAuthorityCursor,
    pub(crate) revision: OperationCatalogueRevision,
    pub(crate) next_sequence: OperationSequence,
    pub(crate) limits: OperationCatalogueLimits,
    pub(crate) operations: Vec<OperationRecord>,
    pub(crate) terminal_eviction_count: u64,
    pub(crate) closed: bool,
}

impl OperationCatalogue {
    /// Constructs an empty catalogue with explicit identity, epoch, and limits.
    #[must_use]
    pub const fn new(
        authority_id: OperationAuthorityId,
        authority_epoch: OperationAuthorityEpoch,
        limits: OperationCatalogueLimits,
    ) -> Self {
        Self {
            authority: OperationAuthorityCursor::new(authority_id, authority_epoch),
            revision: OperationCatalogueRevision::INITIAL,
            next_sequence: OperationSequence::FIRST,
            limits,
            operations: Vec::new(),
            terminal_eviction_count: 0,
            closed: false,
        }
    }

    /// Returns the exact live authority cursor.
    #[must_use]
    pub const fn authority(&self) -> &OperationAuthorityCursor {
        &self.authority
    }

    /// Returns the current catalogue revision.
    #[must_use]
    pub const fn revision(&self) -> OperationCatalogueRevision {
        self.revision
    }

    /// Returns configured finite limits.
    #[must_use]
    pub const fn limits(&self) -> OperationCatalogueLimits {
        self.limits
    }

    /// Returns whether controlled teardown closed the authority.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.closed
    }

    /// Returns all retained records in insertion order.
    pub fn operations(&self) -> impl ExactSizeIterator<Item = &OperationRecord> {
        self.operations.iter()
    }

    /// Returns one retained operation.
    #[must_use]
    pub fn operation(&self, operation_id: &OperationId) -> Option<&OperationRecord> {
        self.operations
            .iter()
            .find(|operation| operation.operation_id() == operation_id)
    }

    /// Returns retained terminal metadata weight.
    pub fn retained_terminal_encoded_weight(&self) -> Result<u64, OperationCatalogueError> {
        terminal_weight(&self.operations)
    }
}
