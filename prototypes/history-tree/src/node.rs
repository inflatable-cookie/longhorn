use longhorn_core::{HistoryEntryId, HistoryRevision};
use longhorn_history::{HistoryEntryMetadata, HistoryEntrySequence};

/// Immutable single-parent node in the private history graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkHistoryNode<P> {
    entry_id: HistoryEntryId,
    parent_entry_id: Option<HistoryEntryId>,
    metadata: HistoryEntryMetadata,
    sequence: HistoryEntrySequence,
    committed_revision: HistoryRevision,
    encoded_weight: u64,
    payload: P,
}

impl<P> ForkHistoryNode<P> {
    pub(crate) const fn new(
        entry_id: HistoryEntryId,
        parent_entry_id: Option<HistoryEntryId>,
        metadata: HistoryEntryMetadata,
        sequence: HistoryEntrySequence,
        committed_revision: HistoryRevision,
        encoded_weight: u64,
        payload: P,
    ) -> Self {
        Self {
            entry_id,
            parent_entry_id,
            metadata,
            sequence,
            committed_revision,
            encoded_weight,
            payload,
        }
    }

    /// Returns the stable entry identity.
    #[must_use]
    pub const fn entry_id(&self) -> &HistoryEntryId {
        &self.entry_id
    }

    /// Returns the immutable parent identity, or the graph root.
    #[must_use]
    pub const fn parent_entry_id(&self) -> Option<&HistoryEntryId> {
        self.parent_entry_id.as_ref()
    }

    /// Returns consumer-owned entry metadata.
    #[must_use]
    pub const fn metadata(&self) -> &HistoryEntryMetadata {
        &self.metadata
    }

    /// Returns the monotonic insertion sequence.
    #[must_use]
    pub const fn sequence(&self) -> HistoryEntrySequence {
        self.sequence
    }

    /// Returns the graph revision that committed the node.
    #[must_use]
    pub const fn committed_revision(&self) -> HistoryRevision {
        self.committed_revision
    }

    /// Returns consumer-measured encoded payload weight.
    #[must_use]
    pub const fn encoded_weight(&self) -> u64 {
        self.encoded_weight
    }

    /// Returns the typed consumer payload.
    #[must_use]
    pub const fn payload(&self) -> &P {
        &self.payload
    }
}
