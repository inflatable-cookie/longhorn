use std::{error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryGroupId, HistoryKindId, HistoryRevision};
use serde::{Deserialize, Serialize};

use crate::HistoryEntrySequence;

/// Defensive hard ceiling for one history label.
pub const MAXIMUM_HISTORY_LABEL_BYTES: usize = 65_536;

/// Nonempty, hard-bounded consumer-owned history label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryLabel(String);

impl HistoryLabel {
    /// Validates and constructs a history label.
    pub fn new(value: impl Into<String>) -> Result<Self, HistoryLabelError> {
        let value = value.into();
        if value.is_empty() {
            return Err(HistoryLabelError::Empty);
        }
        if value.len() > MAXIMUM_HISTORY_LABEL_BYTES {
            return Err(HistoryLabelError::TooLong {
                maximum: MAXIMUM_HISTORY_LABEL_BYTES,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }

    /// Returns the label text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the UTF-8 byte length.
    #[must_use]
    pub fn len_bytes(&self) -> usize {
        self.0.len()
    }
}

/// Invalid history label.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryLabelError {
    /// The label was empty.
    Empty,
    /// The label exceeded the hard byte ceiling.
    TooLong {
        /// Maximum accepted byte length.
        maximum: usize,
        /// Supplied byte length.
        actual: usize,
    },
}

impl fmt::Display for HistoryLabelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("history label cannot be empty"),
            Self::TooLong { maximum, actual } => write!(
                formatter,
                "history label is {actual} bytes; hard maximum is {maximum}"
            ),
        }
    }
}

impl Error for HistoryLabelError {}

/// When a host observed an entry being recorded, in epoch milliseconds.
///
/// Supplied, never derived. These crates own no clock and never will: a
/// history that invented times would assert an observation it never made.
/// A host that has a clock stamps this at record time; one that does not
/// leaves it absent, and its captions show no time.
///
/// Nothing here reads it. Ordering is structural, and no comparison,
/// retention rule or navigation decision may consult this value.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct HistoryRecordedAt(u64);

impl HistoryRecordedAt {
    /// Constructs a recorded-at stamp from epoch milliseconds.
    #[must_use]
    pub const fn from_epoch_millis(value: u64) -> Self {
        Self(value)
    }

    /// Returns the stamp as epoch milliseconds.
    #[must_use]
    pub const fn epoch_millis(self) -> u64 {
        self.0
    }
}

/// Consumer-owned metadata for one history entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEntryMetadata {
    label: HistoryLabel,
    kind_id: Option<HistoryKindId>,
    group_id: Option<HistoryGroupId>,
    recorded_at: Option<HistoryRecordedAt>,
}

impl HistoryEntryMetadata {
    /// Constructs entry metadata.
    #[must_use]
    pub const fn new(
        label: HistoryLabel,
        kind_id: Option<HistoryKindId>,
        group_id: Option<HistoryGroupId>,
    ) -> Self {
        Self {
            label,
            kind_id,
            group_id,
            recorded_at: None,
        }
    }

    /// Returns the metadata with a host-supplied recorded-at stamp.
    ///
    /// A builder rather than a fourth constructor argument: three optional
    /// parameters in a row read as `new(label, None, None, None)` at every
    /// call site, and the stamp is the one a caller most often omits.
    #[must_use]
    pub const fn with_recorded_at(mut self, recorded_at: HistoryRecordedAt) -> Self {
        self.recorded_at = Some(recorded_at);
        self
    }

    /// Returns the user-facing entry label.
    #[must_use]
    pub const fn label(&self) -> &HistoryLabel {
        &self.label
    }

    /// Returns the optional consumer-owned kind.
    #[must_use]
    pub const fn kind_id(&self) -> Option<&HistoryKindId> {
        self.kind_id.as_ref()
    }

    /// Returns the optional explicit committed group.
    #[must_use]
    pub const fn group_id(&self) -> Option<&HistoryGroupId> {
        self.group_id.as_ref()
    }

    /// Returns the optional host-supplied recorded-at stamp.
    #[must_use]
    pub const fn recorded_at(&self) -> Option<HistoryRecordedAt> {
        self.recorded_at
    }

    pub(crate) fn set_group_id(&mut self, group_id: Option<HistoryGroupId>) {
        self.group_id = group_id;
    }
}

/// One retained typed history entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEntry<P> {
    entry_id: HistoryEntryId,
    metadata: HistoryEntryMetadata,
    sequence: HistoryEntrySequence,
    committed_revision: HistoryRevision,
    encoded_weight: u64,
    payload: P,
}

impl<P> HistoryEntry<P> {
    /// Constructs an entry for validated structural state import.
    #[must_use]
    pub const fn new(
        entry_id: HistoryEntryId,
        metadata: HistoryEntryMetadata,
        sequence: HistoryEntrySequence,
        committed_revision: HistoryRevision,
        encoded_weight: u64,
        payload: P,
    ) -> Self {
        Self {
            entry_id,
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

    /// Returns the consumer-owned entry metadata.
    #[must_use]
    pub const fn metadata(&self) -> &HistoryEntryMetadata {
        &self.metadata
    }

    /// Returns the monotonic insertion sequence.
    #[must_use]
    pub const fn sequence(&self) -> HistoryEntrySequence {
        self.sequence
    }

    /// Returns the history revision that last committed this entry.
    #[must_use]
    pub const fn committed_revision(&self) -> HistoryRevision {
        self.committed_revision
    }

    /// Returns the consumer-measured encoded payload weight.
    #[must_use]
    pub const fn encoded_weight(&self) -> u64 {
        self.encoded_weight
    }

    /// Returns the typed consumer payload.
    #[must_use]
    pub const fn payload(&self) -> &P {
        &self.payload
    }

    pub(crate) fn replace(
        &mut self,
        metadata: HistoryEntryMetadata,
        committed_revision: HistoryRevision,
        encoded_weight: u64,
        payload: P,
    ) {
        self.metadata = metadata;
        self.committed_revision = committed_revision;
        self.encoded_weight = encoded_weight;
        self.payload = payload;
    }
}

/// One already-applied product mutation offered for structural recording.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppliedHistoryRecord<P> {
    expected_revision: HistoryRevision,
    entry_id: HistoryEntryId,
    metadata: HistoryEntryMetadata,
    payload: P,
}

impl<P> AppliedHistoryRecord<P> {
    /// Constructs one record request.
    #[must_use]
    pub const fn new(
        expected_revision: HistoryRevision,
        entry_id: HistoryEntryId,
        metadata: HistoryEntryMetadata,
        payload: P,
    ) -> Self {
        Self {
            expected_revision,
            entry_id,
            metadata,
            payload,
        }
    }

    /// Returns the exact admitted history revision.
    #[must_use]
    pub const fn expected_revision(&self) -> HistoryRevision {
        self.expected_revision
    }

    /// Returns the candidate injected entry identity.
    #[must_use]
    pub const fn entry_id(&self) -> &HistoryEntryId {
        &self.entry_id
    }

    /// Returns the candidate metadata.
    #[must_use]
    pub const fn metadata(&self) -> &HistoryEntryMetadata {
        &self.metadata
    }

    /// Returns the typed product payload.
    #[must_use]
    pub const fn payload(&self) -> &P {
        &self.payload
    }

    pub(crate) fn into_parts(self) -> (HistoryRevision, HistoryEntryId, HistoryEntryMetadata, P) {
        (
            self.expected_revision,
            self.entry_id,
            self.metadata,
            self.payload,
        )
    }
}
