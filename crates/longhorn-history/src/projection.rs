use std::{error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryRevision};

use crate::{
    HistoryEntry, HistoryEntrySequence, HistoryLabel, HistoryRetainedBaseline, LinearHistory,
    MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE,
};

/// Public history topology mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryMode {
    /// One applied path and one optional redo path.
    Linear,
}

/// Authoritative location of one projected entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryEntryPosition {
    /// Applied before the current entry.
    Past,
    /// Current applied entry.
    Current,
    /// Retained redo entry.
    Future,
}

/// Payload-free metadata for one retained history entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEntryProjection {
    entry_id: HistoryEntryId,
    label: HistoryLabel,
    kind_id: Option<HistoryKindId>,
    group_id: Option<HistoryGroupId>,
    sequence: HistoryEntrySequence,
    committed_revision: HistoryRevision,
    encoded_weight: u64,
    position: HistoryEntryPosition,
}

impl HistoryEntryProjection {
    fn from_entry<P>(entry: &HistoryEntry<P>, position: HistoryEntryPosition) -> Self {
        Self {
            entry_id: entry.entry_id().clone(),
            label: entry.metadata().label().clone(),
            kind_id: entry.metadata().kind_id().cloned(),
            group_id: entry.metadata().group_id().cloned(),
            sequence: entry.sequence(),
            committed_revision: entry.committed_revision(),
            encoded_weight: entry.encoded_weight(),
            position,
        }
    }

    /// Returns the stable entry identity.
    #[must_use]
    pub const fn entry_id(&self) -> &HistoryEntryId {
        &self.entry_id
    }

    /// Returns the consumer-owned label.
    #[must_use]
    pub const fn label(&self) -> &HistoryLabel {
        &self.label
    }

    /// Returns the optional consumer-owned kind.
    #[must_use]
    pub const fn kind_id(&self) -> Option<&HistoryKindId> {
        self.kind_id.as_ref()
    }

    /// Returns the optional committed group identity.
    #[must_use]
    pub const fn group_id(&self) -> Option<&HistoryGroupId> {
        self.group_id.as_ref()
    }

    /// Returns the insertion sequence.
    #[must_use]
    pub const fn sequence(&self) -> HistoryEntrySequence {
        self.sequence
    }

    /// Returns the last revision that changed the entry.
    #[must_use]
    pub const fn committed_revision(&self) -> HistoryRevision {
        self.committed_revision
    }

    /// Returns the retained encoded payload weight.
    #[must_use]
    pub const fn encoded_weight(&self) -> u64 {
        self.encoded_weight
    }

    /// Returns the authoritative topology position.
    #[must_use]
    pub const fn position(&self) -> HistoryEntryPosition {
        self.position
    }
}

/// Bounded newest-first metadata page request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryPageRequest {
    offset: usize,
    limit: usize,
}

impl HistoryPageRequest {
    /// Validates a page request against the hard public ceiling.
    pub const fn new(offset: usize, limit: usize) -> Result<Self, HistoryPageRequestError> {
        if limit == 0 {
            return Err(HistoryPageRequestError::Zero);
        }
        if limit > MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE {
            return Err(HistoryPageRequestError::TooLarge {
                maximum: MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE,
                actual: limit,
            });
        }
        Ok(Self { offset, limit })
    }

    /// Returns the newest-first entry offset.
    #[must_use]
    pub const fn offset(self) -> usize {
        self.offset
    }

    /// Returns the requested maximum entries.
    #[must_use]
    pub const fn limit(self) -> usize {
        self.limit
    }
}

/// Invalid hard-bounded page request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryPageRequestError {
    /// The requested page size was zero.
    Zero,
    /// The requested page exceeded the hard ceiling.
    TooLarge {
        /// Hard ceiling.
        maximum: usize,
        /// Supplied size.
        actual: usize,
    },
}

impl fmt::Display for HistoryPageRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("history page size must be nonzero"),
            Self::TooLarge { maximum, actual } => write!(
                formatter,
                "history page size is {actual}; hard maximum is {maximum}"
            ),
        }
    }
}

impl Error for HistoryPageRequestError {}

/// Authoritative payload-free history summary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistorySummary {
    history_id: HistoryId,
    revision: HistoryRevision,
    mode: HistoryMode,
    undo_depth: usize,
    redo_depth: usize,
    current_entry_id: Option<HistoryEntryId>,
    next_undo_label: Option<HistoryLabel>,
    next_redo_label: Option<HistoryLabel>,
    retained_entry_count: usize,
    retained_encoded_weight: u64,
    retained_baseline: HistoryRetainedBaseline,
}

impl HistorySummary {
    /// Returns the history authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the current structural revision.
    #[must_use]
    pub const fn revision(&self) -> HistoryRevision {
        self.revision
    }

    /// Returns the public topology mode.
    #[must_use]
    pub const fn mode(&self) -> HistoryMode {
        self.mode
    }

    /// Returns retained applied depth.
    #[must_use]
    pub const fn undo_depth(&self) -> usize {
        self.undo_depth
    }

    /// Returns retained future depth.
    #[must_use]
    pub const fn redo_depth(&self) -> usize {
        self.redo_depth
    }

    /// Returns the current applied entry.
    #[must_use]
    pub const fn current_entry_id(&self) -> Option<&HistoryEntryId> {
        self.current_entry_id.as_ref()
    }

    /// Returns the next undo label.
    #[must_use]
    pub const fn next_undo_label(&self) -> Option<&HistoryLabel> {
        self.next_undo_label.as_ref()
    }

    /// Returns the next redo label.
    #[must_use]
    pub const fn next_redo_label(&self) -> Option<&HistoryLabel> {
        self.next_redo_label.as_ref()
    }

    /// Returns retained entry count.
    #[must_use]
    pub const fn retained_entry_count(&self) -> usize {
        self.retained_entry_count
    }

    /// Returns retained encoded weight.
    #[must_use]
    pub const fn retained_encoded_weight(&self) -> u64 {
        self.retained_encoded_weight
    }

    /// Returns durable retained-baseline evidence.
    #[must_use]
    pub const fn retained_baseline(&self) -> &HistoryRetainedBaseline {
        &self.retained_baseline
    }
}

/// One authoritative newest-first metadata page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryPage {
    history_id: HistoryId,
    revision: HistoryRevision,
    offset: usize,
    total_entries: usize,
    entries: Vec<HistoryEntryProjection>,
    truncated_before: bool,
    truncated_after: bool,
    retained_baseline: HistoryRetainedBaseline,
}

impl HistoryPage {
    /// Returns the history authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the exact projected revision.
    #[must_use]
    pub const fn revision(&self) -> HistoryRevision {
        self.revision
    }

    /// Returns the newest-first page offset.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Returns total authoritative retained entries.
    #[must_use]
    pub const fn total_entries(&self) -> usize {
        self.total_entries
    }

    /// Returns payload-free projected entries.
    #[must_use]
    pub fn entries(&self) -> &[HistoryEntryProjection] {
        &self.entries
    }

    /// Returns whether newer entries precede this page.
    #[must_use]
    pub const fn truncated_before(&self) -> bool {
        self.truncated_before
    }

    /// Returns whether older entries follow this page.
    #[must_use]
    pub const fn truncated_after(&self) -> bool {
        self.truncated_after
    }

    /// Returns durable baseline evidence for older applied history.
    #[must_use]
    pub const fn retained_baseline(&self) -> &HistoryRetainedBaseline {
        &self.retained_baseline
    }
}

impl<P> LinearHistory<P> {
    /// Projects one payload-free authoritative summary.
    pub fn project_summary(&self) -> Result<HistorySummary, HistoryProjectionError> {
        Ok(HistorySummary {
            history_id: self.state.history_id.clone(),
            revision: self.state.revision,
            mode: HistoryMode::Linear,
            undo_depth: self.state.applied.len(),
            redo_depth: self.state.future.len(),
            current_entry_id: self
                .state
                .applied
                .last()
                .map(|entry| entry.entry_id().clone()),
            next_undo_label: self
                .state
                .applied
                .last()
                .map(|entry| entry.metadata().label().clone()),
            next_redo_label: self
                .state
                .future
                .last()
                .map(|entry| entry.metadata().label().clone()),
            retained_entry_count: self.state.applied.len() + self.state.future.len(),
            retained_encoded_weight: self
                .retained_encoded_weight()
                .map_err(HistoryProjectionError::Retention)?,
            retained_baseline: self.state.retained_baseline.clone(),
        })
    }

    /// Projects one bounded newest-first payload-free metadata page.
    pub fn project_page(
        &self,
        request: HistoryPageRequest,
    ) -> Result<HistoryPage, HistoryProjectionError> {
        if request.limit > self.projection_limits.maximum_page_size() {
            return Err(HistoryProjectionError::PageTooLarge {
                maximum: self.projection_limits.maximum_page_size(),
                actual: request.limit,
            });
        }
        let total_entries = self.state.applied.len() + self.state.future.len();
        if request.offset > total_entries {
            return Err(HistoryProjectionError::OffsetOutOfRange {
                maximum: total_entries,
                actual: request.offset,
            });
        }
        let entries = self
            .state
            .future
            .iter()
            .map(|entry| HistoryEntryProjection::from_entry(entry, HistoryEntryPosition::Future))
            .chain(
                self.state
                    .applied
                    .iter()
                    .rev()
                    .enumerate()
                    .map(|(index, entry)| {
                        HistoryEntryProjection::from_entry(
                            entry,
                            if index == 0 {
                                HistoryEntryPosition::Current
                            } else {
                                HistoryEntryPosition::Past
                            },
                        )
                    }),
            )
            .skip(request.offset)
            .take(request.limit)
            .collect::<Vec<_>>();
        let page_end = request.offset + entries.len();
        Ok(HistoryPage {
            history_id: self.state.history_id.clone(),
            revision: self.state.revision,
            offset: request.offset,
            total_entries,
            entries,
            truncated_before: request.offset != 0,
            truncated_after: page_end < total_entries,
            retained_baseline: self.state.retained_baseline.clone(),
        })
    }
}

/// Rejected authoritative projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryProjectionError {
    /// The request exceeded this authority's configured page size.
    PageTooLarge {
        /// Configured maximum.
        maximum: usize,
        /// Requested size.
        actual: usize,
    },
    /// The newest-first offset exceeded retained history.
    OffsetOutOfRange {
        /// Maximum accepted offset.
        maximum: usize,
        /// Requested offset.
        actual: usize,
    },
    /// Retained weight evidence was invalid.
    Retention(crate::HistoryRetentionError),
}

impl fmt::Display for HistoryProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PageTooLarge { maximum, actual } => write!(
                formatter,
                "history page size is {actual}; configured maximum is {maximum}"
            ),
            Self::OffsetOutOfRange { maximum, actual } => write!(
                formatter,
                "history page offset is {actual}; maximum is {maximum}"
            ),
            Self::Retention(error) => write!(formatter, "history projection failed: {error}"),
        }
    }
}

impl Error for HistoryProjectionError {}
