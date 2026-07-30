use longhorn_core::{HistoryEntryId, HistoryId, HistoryKindId, HistoryRevision};
use longhorn_history::{
    AppliedHistoryRecord, HistoryEntry, HistoryEntryMetadata, HistoryEntrySequence, HistoryLabel,
};

pub(crate) fn history_id(value: &str) -> HistoryId {
    HistoryId::new(value).expect("fixture history id")
}

pub(crate) fn entry_id(value: &str) -> HistoryEntryId {
    HistoryEntryId::new(value).expect("fixture entry id")
}

pub(crate) fn metadata(label: &str, kind: &str) -> HistoryEntryMetadata {
    HistoryEntryMetadata::new(
        HistoryLabel::new(label).expect("fixture label"),
        Some(HistoryKindId::new(kind).expect("fixture kind")),
        None,
    )
}

pub(crate) fn record<P>(
    revision: u64,
    id: &str,
    metadata: HistoryEntryMetadata,
    payload: P,
) -> AppliedHistoryRecord<P> {
    AppliedHistoryRecord::new(
        HistoryRevision::new(revision),
        entry_id(id),
        metadata,
        payload,
    )
}

pub(crate) fn entry<P>(
    id: &str,
    label: &str,
    kind: &str,
    sequence: u64,
    revision: u64,
    payload: P,
) -> HistoryEntry<P> {
    HistoryEntry::new(
        entry_id(id),
        metadata(label, kind),
        HistoryEntrySequence::new(sequence).expect("fixture sequence"),
        HistoryRevision::new(revision),
        1,
        payload,
    )
}
