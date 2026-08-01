use std::{convert::Infallible, error::Error, fmt};

use longhorn_core::{HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryRevision};
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::Value;

use crate::{
    HistoryCommittedTransition, HistoryCommittedTransitionKind, HistoryDiscardReason, HistoryEntry,
    HistoryEntryMetadata, HistoryEntrySequence, HistoryLabel, HistoryLimits, HistoryLimitsError,
    HistoryNavigationLimits, HistoryPolicy, HistoryProjectionLimits, HistoryRetainedBaseline,
    HistoryStateError, LinearHistory, LinearHistoryState, MAXIMUM_HISTORY_ENCODED_WEIGHT,
};

/// Stable structural format family for public linear history envelopes.
pub const HISTORY_FORMAT_FAMILY: &str = "longhorn.linear-history";
/// Current structural envelope version.
pub const CURRENT_HISTORY_STRUCTURAL_VERSION: u32 = 1;
/// Defensive hard ceiling for one encoded structural envelope.
pub const MAXIMUM_HISTORY_PERSISTED_BYTES: u64 = 1 << 41;
/// Defensive hard ceiling for one payload codec family identity.
pub const MAXIMUM_HISTORY_PAYLOAD_CODEC_FAMILY_BYTES: usize = 128;

/// Registered, bounded consumer payload codec family.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct HistoryPayloadCodecFamily(String);

impl HistoryPayloadCodecFamily {
    /// Validates and constructs one stable codec family.
    pub fn new(value: impl Into<String>) -> Result<Self, HistoryPayloadCodecFamilyError> {
        let value = value.into();
        if value.is_empty() {
            return Err(HistoryPayloadCodecFamilyError::Empty);
        }
        if value.len() > MAXIMUM_HISTORY_PAYLOAD_CODEC_FAMILY_BYTES {
            return Err(HistoryPayloadCodecFamilyError::TooLong {
                maximum: MAXIMUM_HISTORY_PAYLOAD_CODEC_FAMILY_BYTES,
                actual: value.len(),
            });
        }
        if let Some((index, _)) = value.char_indices().find(|(_, character)| {
            !(character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '_' | ':' | '-'))
        }) {
            return Err(HistoryPayloadCodecFamilyError::InvalidCharacter { index });
        }
        Ok(Self(value))
    }

    /// Returns the serialized codec family.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HistoryPayloadCodecFamily {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for HistoryPayloadCodecFamily {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// Invalid payload codec family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryPayloadCodecFamilyError {
    /// The family was empty.
    Empty,
    /// The family exceeded the defensive byte ceiling.
    TooLong {
        /// Defensive ceiling.
        maximum: usize,
        /// Supplied bytes.
        actual: usize,
    },
    /// One character fell outside the stable lowercase ASCII grammar.
    InvalidCharacter {
        /// Invalid byte index.
        index: usize,
    },
}

impl fmt::Display for HistoryPayloadCodecFamilyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("history payload codec family cannot be empty"),
            Self::TooLong { maximum, actual } => write!(
                formatter,
                "history payload codec family is {actual} bytes; maximum is {maximum}"
            ),
            Self::InvalidCharacter { index } => write!(
                formatter,
                "history payload codec family has an invalid character at byte {index}"
            ),
        }
    }
}

impl Error for HistoryPayloadCodecFamilyError {}

/// Independent version of one consumer payload codec family.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct HistoryPayloadCodecVersion(u32);

impl HistoryPayloadCodecVersion {
    /// Constructs one codec version.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the serialized version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// One exact step produced by a registered payload migration hook.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryPayloadMigrationStep {
    version: HistoryPayloadCodecVersion,
    bytes: Vec<u8>,
}

impl HistoryPayloadMigrationStep {
    /// Constructs one migration step.
    #[must_use]
    pub const fn new(version: HistoryPayloadCodecVersion, bytes: Vec<u8>) -> Self {
        Self { version, bytes }
    }

    /// Returns the version produced by this step.
    #[must_use]
    pub const fn version(&self) -> HistoryPayloadCodecVersion {
        self.version
    }

    /// Returns migrated payload bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn into_parts(self) -> (HistoryPayloadCodecVersion, Vec<u8>) {
        (self.version, self.bytes)
    }
}

/// Current registered payload codec authority exposed to migration hooks.
#[derive(Clone, Copy, Debug)]
pub struct HistoryPayloadMigrationTarget<'target> {
    family: &'target HistoryPayloadCodecFamily,
    version: HistoryPayloadCodecVersion,
}

impl<'target> HistoryPayloadMigrationTarget<'target> {
    /// Returns the registered codec family.
    #[must_use]
    pub const fn family(self) -> &'target HistoryPayloadCodecFamily {
        self.family
    }

    /// Returns the registered current codec version.
    #[must_use]
    pub const fn version(self) -> HistoryPayloadCodecVersion {
        self.version
    }
}

/// Registered consumer codec for one typed payload family.
pub trait HistoryPayloadCodec<P> {
    /// Codec or payload migration failure.
    type Error;

    /// Returns the stable registered codec family.
    fn family(&self) -> &HistoryPayloadCodecFamily;

    /// Returns the current codec version.
    fn version(&self) -> HistoryPayloadCodecVersion;

    /// Encodes one current typed payload.
    fn encode(&self, payload: &P) -> Result<Vec<u8>, Self::Error>;

    /// Decodes one payload at the current version.
    fn decode(&self, bytes: &[u8]) -> Result<P, Self::Error>;

    /// Migrates one older payload version or returns `None` when unsupported.
    fn migrate_one(
        &self,
        _from: HistoryPayloadCodecVersion,
        _bytes: Vec<u8>,
        _target: HistoryPayloadMigrationTarget<'_>,
    ) -> Result<Option<HistoryPayloadMigrationStep>, Self::Error> {
        Ok(None)
    }
}

/// One exact step produced by a structural envelope migration hook.
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryStructuralMigrationStep {
    version: u32,
    document: Value,
}

impl HistoryStructuralMigrationStep {
    /// Constructs one structural migration step.
    #[must_use]
    pub const fn new(version: u32, document: Value) -> Self {
        Self { version, document }
    }

    /// Returns the structural version produced by this step.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Returns the migrated raw structural document.
    #[must_use]
    pub const fn document(&self) -> &Value {
        &self.document
    }

    fn into_parts(self) -> (u32, Value) {
        (self.version, self.document)
    }
}

/// Current Longhorn structural authority exposed to migration hooks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryStructuralMigrationTarget {
    version: u32,
}

impl HistoryStructuralMigrationTarget {
    /// Returns the stable structural format family.
    #[must_use]
    pub const fn family(self) -> &'static str {
        HISTORY_FORMAT_FAMILY
    }

    /// Returns the current structural version.
    #[must_use]
    pub const fn version(self) -> u32 {
        self.version
    }
}

/// Registered one-step migration for older structural envelopes.
pub trait HistoryStructuralMigration {
    /// Structural migration failure.
    type Error;

    /// Migrates one older structural document or returns `None`.
    fn migrate_one(
        &self,
        from: u32,
        document: Value,
        target: HistoryStructuralMigrationTarget,
    ) -> Result<Option<HistoryStructuralMigrationStep>, Self::Error>;
}

/// Explicit registration with no older structural migration.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoHistoryStructuralMigration;

impl HistoryStructuralMigration for NoHistoryStructuralMigration {
    type Error = Infallible;

    fn migrate_one(
        &self,
        _from: u32,
        _document: Value,
        _target: HistoryStructuralMigrationTarget,
    ) -> Result<Option<HistoryStructuralMigrationStep>, Self::Error> {
        Ok(None)
    }
}

/// Caller-selected bound for untrusted structural bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryPersistenceLimits {
    maximum_envelope_bytes: u64,
}

impl HistoryPersistenceLimits {
    /// Validates one explicit load and encode bound.
    pub const fn new(maximum_envelope_bytes: u64) -> Result<Self, HistoryPersistenceLimitsError> {
        if maximum_envelope_bytes == 0 {
            return Err(HistoryPersistenceLimitsError::Zero);
        }
        if maximum_envelope_bytes > MAXIMUM_HISTORY_PERSISTED_BYTES {
            return Err(HistoryPersistenceLimitsError::TooLarge {
                maximum: MAXIMUM_HISTORY_PERSISTED_BYTES,
                actual: maximum_envelope_bytes,
            });
        }
        Ok(Self {
            maximum_envelope_bytes,
        })
    }

    /// Returns the maximum accepted or produced envelope size.
    #[must_use]
    pub const fn maximum_envelope_bytes(self) -> u64 {
        self.maximum_envelope_bytes
    }
}

/// Invalid structural persistence bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryPersistenceLimitsError {
    /// The bound was zero.
    Zero,
    /// The bound exceeded the defensive ceiling.
    TooLarge {
        /// Defensive ceiling.
        maximum: u64,
        /// Supplied bound.
        actual: u64,
    },
}

impl fmt::Display for HistoryPersistenceLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("history persistence bound must be nonzero"),
            Self::TooLarge { maximum, actual } => write!(
                formatter,
                "history persistence bound is {actual}; hard maximum is {maximum}"
            ),
        }
    }
}

impl Error for HistoryPersistenceLimitsError {}

/// Registered structural and payload persistence authority.
#[derive(Clone, Debug)]
pub struct HistoryPersistence<C, M> {
    codec: C,
    structural_migration: M,
    limits: HistoryPersistenceLimits,
}

impl<C, M> HistoryPersistence<C, M> {
    /// Registers a codec, structural migration hook, and byte bound.
    #[must_use]
    pub const fn new(codec: C, structural_migration: M, limits: HistoryPersistenceLimits) -> Self {
        Self {
            codec,
            structural_migration,
            limits,
        }
    }

    /// Returns the configured untrusted-byte bound.
    #[must_use]
    pub const fn limits(&self) -> HistoryPersistenceLimits {
        self.limits
    }
}

impl<C> HistoryPersistence<C, NoHistoryStructuralMigration> {
    /// Registers a codec with no older structural migration.
    #[must_use]
    pub const fn without_structural_migration(codec: C, limits: HistoryPersistenceLimits) -> Self {
        Self::new(codec, NoHistoryStructuralMigration, limits)
    }
}

impl<C, M> HistoryPersistence<C, M>
where
    M: HistoryStructuralMigration,
{
    /// Encodes one complete, validated linear authority.
    pub fn encode<P>(
        &self,
        history: &LinearHistory<P>,
    ) -> Result<Vec<u8>, HistoryEncodeError<C::Error>>
    where
        C: HistoryPayloadCodec<P>,
    {
        let mut entries = Vec::with_capacity(history.applied().len() + history.future().len());
        for entry in history
            .applied()
            .iter()
            .chain(history.future().iter().rev())
        {
            let payload = self.codec.encode(entry.payload()).map_err(|error| {
                HistoryEncodeError::Payload {
                    entry_id: entry.entry_id().clone(),
                    error,
                }
            })?;
            let actual =
                u64::try_from(payload.len()).map_err(|_| HistoryEncodeError::SizeOverflow)?;
            if actual != entry.encoded_weight() {
                return Err(HistoryEncodeError::PayloadWeightMismatch {
                    entry_id: entry.entry_id().clone(),
                    recorded: entry.encoded_weight(),
                    actual,
                });
            }
            entries.push(PersistedHistoryEntry {
                entry_id: entry.entry_id().clone(),
                label: entry.metadata().label().as_str().to_owned(),
                kind_id: entry.metadata().kind_id().cloned(),
                group_id: entry.metadata().group_id().cloned(),
                sequence: entry.sequence().get(),
                committed_revision: entry.committed_revision(),
                encoded_weight: entry.encoded_weight(),
                payload,
            });
        }

        let maximum_entries = u64::try_from(history.limits().maximum_entries())
            .map_err(|_| HistoryEncodeError::SizeOverflow)?;
        let maximum_label_bytes = u64::try_from(history.limits().maximum_label_bytes())
            .map_err(|_| HistoryEncodeError::SizeOverflow)?;
        let current_position =
            u64::try_from(history.applied().len()).map_err(|_| HistoryEncodeError::SizeOverflow)?;
        let envelope = PersistedLinearHistoryEnvelope {
            format_family: HISTORY_FORMAT_FAMILY.to_owned(),
            structural_version: CURRENT_HISTORY_STRUCTURAL_VERSION,
            payload_codec: PersistedPayloadCodec {
                family: self.codec.family().clone(),
                version: self.codec.version(),
            },
            mode: PersistedHistoryMode::Linear,
            history_id: history.history_id().clone(),
            revision: history.revision(),
            limits: PersistedHistoryLimits {
                maximum_entries,
                maximum_encoded_weight: history.limits().maximum_encoded_weight(),
                maximum_label_bytes,
            },
            next_sequence: history.next_sequence().get(),
            retained_baseline: PersistedRetainedBaseline::from(history.retained_baseline()),
            current_position,
            entries,
        };
        let bytes = serde_json::to_vec_pretty(&envelope).map_err(HistoryEncodeError::Structural)?;
        validate_envelope_size(bytes.len(), self.limits.maximum_envelope_bytes()).map_err(
            |(maximum, actual)| HistoryEncodeError::EnvelopeTooLarge { maximum, actual },
        )?;
        Ok(bytes)
    }

    /// Loads and validates a complete authority with default transient limits.
    pub fn load<P, T>(
        &self,
        expected_history_id: &HistoryId,
        bytes: &[u8],
        policy: &T,
    ) -> HistoryLoadAttempt<P, C::Error, T::Error, M::Error>
    where
        C: HistoryPayloadCodec<P>,
        T: HistoryPolicy<P>,
    {
        self.load_with_runtime_limits(
            expected_history_id,
            bytes,
            HistoryNavigationLimits::DEFAULT,
            HistoryProjectionLimits::DEFAULT,
            policy,
        )
    }

    /// Loads and validates a complete authority with explicit transient limits.
    pub fn load_with_runtime_limits<P, T>(
        &self,
        expected_history_id: &HistoryId,
        bytes: &[u8],
        navigation_limits: HistoryNavigationLimits,
        projection_limits: HistoryProjectionLimits,
        policy: &T,
    ) -> HistoryLoadAttempt<P, C::Error, T::Error, M::Error>
    where
        C: HistoryPayloadCodec<P>,
        T: HistoryPolicy<P>,
    {
        validate_envelope_size(bytes.len(), self.limits.maximum_envelope_bytes())
            .map_err(|(maximum, actual)| HistoryLoadError::EnvelopeTooLarge { maximum, actual })?;
        let mut document: Value =
            serde_json::from_slice(bytes).map_err(HistoryLoadError::InvalidJson)?;
        let header = read_structural_header(&document).map_err(HistoryLoadError::InvalidHeader)?;
        if header.format_family != HISTORY_FORMAT_FAMILY {
            return Err(HistoryLoadError::ForeignFormatFamily {
                actual: header.format_family,
            });
        }
        if header.structural_version > CURRENT_HISTORY_STRUCTURAL_VERSION {
            return Err(HistoryLoadError::FutureStructuralVersion {
                actual: header.structural_version,
                maximum: CURRENT_HISTORY_STRUCTURAL_VERSION,
            });
        }
        let source_structural_version = header.structural_version;
        let target = HistoryStructuralMigrationTarget {
            version: CURRENT_HISTORY_STRUCTURAL_VERSION,
        };
        let mut structural_version = source_structural_version;
        while structural_version < CURRENT_HISTORY_STRUCTURAL_VERSION {
            let expected = structural_version.checked_add(1).ok_or(
                HistoryLoadError::InvalidStructuralMigration {
                    from: structural_version,
                    produced: structural_version,
                },
            )?;
            let step = self
                .structural_migration
                .migrate_one(structural_version, document, target)
                .map_err(HistoryLoadError::StructuralMigration)?
                .ok_or(HistoryLoadError::MissingStructuralMigration {
                    from: structural_version,
                })?;
            let (produced, migrated) = step.into_parts();
            if produced != expected {
                return Err(HistoryLoadError::InvalidStructuralMigration {
                    from: structural_version,
                    produced,
                });
            }
            let migrated_header =
                read_structural_header(&migrated).map_err(HistoryLoadError::InvalidHeader)?;
            if migrated_header.format_family != HISTORY_FORMAT_FAMILY
                || migrated_header.structural_version != produced
            {
                return Err(HistoryLoadError::InvalidStructuralMigration {
                    from: structural_version,
                    produced: migrated_header.structural_version,
                });
            }
            structural_version = produced;
            document = migrated;
        }

        let envelope: PersistedLinearHistoryEnvelope =
            serde_json::from_value(document).map_err(HistoryLoadError::InvalidEnvelope)?;
        if envelope.format_family != HISTORY_FORMAT_FAMILY
            || envelope.structural_version != CURRENT_HISTORY_STRUCTURAL_VERSION
        {
            return Err(HistoryLoadError::InvalidHeader(
                HistoryStructuralHeaderError::InvalidVersion,
            ));
        }
        if &envelope.history_id != expected_history_id {
            return Err(HistoryLoadError::ForeignHistory {
                expected: expected_history_id.clone(),
                actual: envelope.history_id,
            });
        }
        if &envelope.payload_codec.family != self.codec.family() {
            return Err(HistoryLoadError::ForeignPayloadCodecFamily {
                expected: self.codec.family().clone(),
                actual: envelope.payload_codec.family,
            });
        }
        if envelope.payload_codec.version > self.codec.version() {
            return Err(HistoryLoadError::FuturePayloadCodecVersion {
                actual: envelope.payload_codec.version,
                maximum: self.codec.version(),
            });
        }

        let limits = decode_limits(envelope.limits).map_err(HistoryLoadError::Limits)?;
        let current_position = usize::try_from(envelope.current_position)
            .map_err(|_| HistoryLoadError::PositionOverflow)?;
        if current_position > envelope.entries.len() {
            return Err(HistoryLoadError::InvalidCurrentPosition {
                entries: envelope.entries.len(),
                actual: current_position,
            });
        }
        let next_sequence = HistoryEntrySequence::new(envelope.next_sequence)
            .map_err(|_| HistoryLoadError::InvalidNextSequence)?;
        let retained_baseline = decode_baseline(envelope.retained_baseline)
            .map_err(|_| HistoryLoadError::InvalidBaselineSequence)?;
        let source_payload_codec_version = envelope.payload_codec.version;
        let payload_target = HistoryPayloadMigrationTarget {
            family: self.codec.family(),
            version: self.codec.version(),
        };

        let mut retained_weight = 0_u64;
        let mut entries = Vec::with_capacity(envelope.entries.len());
        for entry in envelope.entries {
            let source_weight =
                u64::try_from(entry.payload.len()).map_err(|_| HistoryLoadError::SizeOverflow)?;
            if source_weight != entry.encoded_weight {
                return Err(HistoryLoadError::PayloadWeightMismatch {
                    entry_id: entry.entry_id,
                    recorded: entry.encoded_weight,
                    actual: source_weight,
                });
            }

            let entry_id = entry.entry_id;
            let mut payload_version = source_payload_codec_version;
            let mut payload_bytes = entry.payload;
            while payload_version < self.codec.version() {
                let expected = payload_version.checked_next().ok_or(
                    HistoryLoadError::InvalidPayloadMigration {
                        entry_id: entry_id.clone(),
                        from: payload_version,
                        produced: payload_version,
                    },
                )?;
                let step = self
                    .codec
                    .migrate_one(payload_version, payload_bytes, payload_target)
                    .map_err(|error| HistoryLoadError::PayloadMigration {
                        entry_id: entry_id.clone(),
                        error,
                    })?
                    .ok_or_else(|| HistoryLoadError::MissingPayloadMigration {
                        entry_id: entry_id.clone(),
                        from: payload_version,
                    })?;
                let (produced, migrated) = step.into_parts();
                if produced != expected {
                    return Err(HistoryLoadError::InvalidPayloadMigration {
                        entry_id,
                        from: payload_version,
                        produced,
                    });
                }
                payload_version = produced;
                payload_bytes = migrated;
            }

            let encoded_weight =
                u64::try_from(payload_bytes.len()).map_err(|_| HistoryLoadError::SizeOverflow)?;
            if encoded_weight > limits.maximum_encoded_weight()
                || encoded_weight > MAXIMUM_HISTORY_ENCODED_WEIGHT
            {
                return Err(HistoryLoadError::PayloadTooHeavy {
                    entry_id,
                    maximum: limits.maximum_encoded_weight(),
                    actual: encoded_weight,
                });
            }
            retained_weight = retained_weight
                .checked_add(encoded_weight)
                .ok_or(HistoryLoadError::SizeOverflow)?;
            if retained_weight > limits.maximum_encoded_weight() {
                return Err(HistoryLoadError::RetainedWeightTooLarge {
                    maximum: limits.maximum_encoded_weight(),
                    actual: retained_weight,
                });
            }

            let payload =
                self.codec
                    .decode(&payload_bytes)
                    .map_err(|error| HistoryLoadError::Payload {
                        entry_id: entry_id.clone(),
                        error,
                    })?;
            if policy.is_noop(&payload) {
                return Err(HistoryLoadError::NoOpPayload(entry_id));
            }
            policy
                .inverse(&payload)
                .map_err(|error| HistoryLoadError::Policy {
                    entry_id: entry_id.clone(),
                    error,
                })?;
            let policy_weight =
                policy
                    .encoded_weight(&payload)
                    .map_err(|error| HistoryLoadError::Policy {
                        entry_id: entry_id.clone(),
                        error,
                    })?;
            if policy_weight != encoded_weight {
                return Err(HistoryLoadError::PolicyWeightMismatch {
                    entry_id,
                    codec: encoded_weight,
                    policy: policy_weight,
                });
            }

            let label =
                HistoryLabel::new(entry.label).map_err(|error| HistoryLoadError::InvalidLabel {
                    entry_id: entry_id.clone(),
                    error,
                })?;
            let sequence = HistoryEntrySequence::new(entry.sequence).map_err(|_| {
                HistoryLoadError::InvalidEntrySequence {
                    entry_id: entry_id.clone(),
                }
            })?;
            entries.push(HistoryEntry::new(
                entry_id,
                HistoryEntryMetadata::new(label, entry.kind_id, entry.group_id),
                sequence,
                entry.committed_revision,
                encoded_weight,
                payload,
            ));
        }

        let future_canonical = entries.split_off(current_position);
        let applied = entries;
        let future = future_canonical.into_iter().rev().collect::<Vec<_>>();
        let state = LinearHistoryState::with_retained_baseline(
            expected_history_id.clone(),
            envelope.revision,
            next_sequence,
            retained_baseline,
            applied,
            future,
        );
        let history = LinearHistory::from_state_with_runtime_limits(
            limits,
            navigation_limits,
            projection_limits,
            state,
        )
        .map_err(HistoryLoadError::State)?;
        let applied_entries =
            u64::try_from(history.applied().len()).map_err(|_| HistoryLoadError::SizeOverflow)?;
        let future_entries =
            u64::try_from(history.future().len()).map_err(|_| HistoryLoadError::SizeOverflow)?;
        let outcome = if source_structural_version == CURRENT_HISTORY_STRUCTURAL_VERSION
            && source_payload_codec_version == self.codec.version()
        {
            HistoryLoadOutcome::Preserved
        } else {
            HistoryLoadOutcome::Migrated {
                structural: source_structural_version != CURRENT_HISTORY_STRUCTURAL_VERSION,
                payload: source_payload_codec_version != self.codec.version(),
            }
        };
        let transition = HistoryCommittedTransition::new(
            expected_history_id.clone(),
            None,
            history.revision(),
            HistoryCommittedTransitionKind::Imported {
                source_structural_version,
                structural_version: CURRENT_HISTORY_STRUCTURAL_VERSION,
                payload_codec_family: self.codec.family().as_str().to_owned(),
                source_payload_codec_version: source_payload_codec_version.get(),
                payload_codec_version: self.codec.version().get(),
                applied_entries,
                future_entries,
            },
        );
        Ok(HistoryLoadResult {
            history,
            receipt: HistoryLoadReceipt {
                outcome,
                source_structural_version,
                structural_version: CURRENT_HISTORY_STRUCTURAL_VERSION,
                payload_codec_family: self.codec.family().clone(),
                source_payload_codec_version,
                payload_codec_version: self.codec.version(),
                transition,
            },
        })
    }
}

/// Visible successful compatibility outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryLoadOutcome {
    /// Current structural and payload versions were preserved.
    Preserved,
    /// One or both independent version families migrated.
    Migrated {
        /// Structural migration ran.
        structural: bool,
        /// Payload migration ran.
        payload: bool,
    },
}

/// Successful checked load receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryLoadReceipt {
    outcome: HistoryLoadOutcome,
    source_structural_version: u32,
    structural_version: u32,
    payload_codec_family: HistoryPayloadCodecFamily,
    source_payload_codec_version: HistoryPayloadCodecVersion,
    payload_codec_version: HistoryPayloadCodecVersion,
    transition: HistoryCommittedTransition,
}

impl HistoryLoadReceipt {
    /// Returns whether bytes were preserved or visibly migrated.
    #[must_use]
    pub const fn outcome(&self) -> HistoryLoadOutcome {
        self.outcome
    }

    /// Returns the structural version found in source bytes.
    #[must_use]
    pub const fn source_structural_version(&self) -> u32 {
        self.source_structural_version
    }

    /// Returns the accepted structural version.
    #[must_use]
    pub const fn structural_version(&self) -> u32 {
        self.structural_version
    }

    /// Returns the registered payload codec family.
    #[must_use]
    pub const fn payload_codec_family(&self) -> &HistoryPayloadCodecFamily {
        &self.payload_codec_family
    }

    /// Returns the payload codec version found in source bytes.
    #[must_use]
    pub const fn source_payload_codec_version(&self) -> HistoryPayloadCodecVersion {
        self.source_payload_codec_version
    }

    /// Returns the accepted payload codec version.
    #[must_use]
    pub const fn payload_codec_version(&self) -> HistoryPayloadCodecVersion {
        self.payload_codec_version
    }

    /// Returns the committed import transition.
    #[must_use]
    pub const fn transition(&self) -> &HistoryCommittedTransition {
        &self.transition
    }
}

/// Fully validated authority plus its visible load receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryLoadResult<P> {
    history: LinearHistory<P>,
    receipt: HistoryLoadReceipt,
}

/// Checked load result across codec, policy, and structural migration hooks.
pub type HistoryLoadAttempt<P, CE, PE, ME> =
    Result<HistoryLoadResult<P>, HistoryLoadError<CE, PE, ME>>;

impl<P> HistoryLoadResult<P> {
    /// Returns the validated authority.
    #[must_use]
    pub const fn history(&self) -> &LinearHistory<P> {
        &self.history
    }

    /// Returns the preserve or migration receipt.
    #[must_use]
    pub const fn receipt(&self) -> &HistoryLoadReceipt {
        &self.receipt
    }

    /// Consumes the result into the validated authority and receipt.
    #[must_use]
    pub fn into_parts(self) -> (LinearHistory<P>, HistoryLoadReceipt) {
        (self.history, self.receipt)
    }
}

/// Explicit discard-history recovery receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryDiscardRecoveryReceipt {
    history_id: HistoryId,
    reason: HistoryDiscardReason,
    transition: HistoryCommittedTransition,
}

impl HistoryDiscardRecoveryReceipt {
    /// Returns the replacement authority identity.
    #[must_use]
    pub const fn history_id(&self) -> &HistoryId {
        &self.history_id
    }

    /// Returns the caller-owned visible discard reason.
    #[must_use]
    pub const fn reason(&self) -> HistoryDiscardReason {
        self.reason
    }

    /// Returns the committed discard transition.
    #[must_use]
    pub const fn transition(&self) -> &HistoryCommittedTransition {
        &self.transition
    }
}

/// Explicit fresh authority produced after a visible discard decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryDiscardRecovery<P> {
    history: LinearHistory<P>,
    receipt: HistoryDiscardRecoveryReceipt,
}

impl<P> HistoryDiscardRecovery<P> {
    /// Returns the fresh empty authority.
    #[must_use]
    pub const fn history(&self) -> &LinearHistory<P> {
        &self.history
    }

    /// Returns explicit discard evidence.
    #[must_use]
    pub const fn receipt(&self) -> &HistoryDiscardRecoveryReceipt {
        &self.receipt
    }

    /// Consumes the recovery into the fresh authority and receipt.
    #[must_use]
    pub fn into_parts(self) -> (LinearHistory<P>, HistoryDiscardRecoveryReceipt) {
        (self.history, self.receipt)
    }
}

/// Deliberately discards unusable persisted history and creates a fresh authority.
#[must_use]
pub fn discard_persisted_history<P>(
    history_id: HistoryId,
    limits: HistoryLimits,
    navigation_limits: HistoryNavigationLimits,
    projection_limits: HistoryProjectionLimits,
    reason: HistoryDiscardReason,
) -> HistoryDiscardRecovery<P> {
    let history = LinearHistory::with_runtime_limits(
        history_id.clone(),
        limits,
        navigation_limits,
        projection_limits,
    );
    let transition = HistoryCommittedTransition::new(
        history_id.clone(),
        None,
        HistoryRevision::INITIAL,
        HistoryCommittedTransitionKind::DiscardedPersistence { reason },
    );
    HistoryDiscardRecovery {
        history,
        receipt: HistoryDiscardRecoveryReceipt {
            history_id,
            reason,
            transition,
        },
    }
}

/// Failed structural encoding.
#[derive(Debug)]
pub enum HistoryEncodeError<E> {
    /// Consumer payload encoding failed.
    Payload {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Consumer codec failure.
        error: E,
    },
    /// Encoded bytes disagreed with the admitted exact payload weight.
    PayloadWeightMismatch {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Weight retained by the authority.
        recorded: u64,
        /// Exact codec byte length.
        actual: u64,
    },
    /// Structural JSON encoding failed.
    Structural(serde_json::Error),
    /// One platform-size conversion overflowed.
    SizeOverflow,
    /// Encoded structural bytes exceeded the explicit caller bound.
    EnvelopeTooLarge {
        /// Configured maximum.
        maximum: u64,
        /// Encoded bytes.
        actual: u64,
    },
}

impl<E: fmt::Display> fmt::Display for HistoryEncodeError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Payload { entry_id, error } => {
                write!(formatter, "cannot encode history entry {entry_id}: {error}")
            }
            Self::PayloadWeightMismatch {
                entry_id,
                recorded,
                actual,
            } => write!(
                formatter,
                "history entry {entry_id} recorded weight {recorded}; codec produced {actual} bytes"
            ),
            Self::Structural(error) => write!(formatter, "cannot encode history envelope: {error}"),
            Self::SizeOverflow => formatter.write_str("history encoded size overflowed"),
            Self::EnvelopeTooLarge { maximum, actual } => write!(
                formatter,
                "history envelope is {actual} bytes; configured maximum is {maximum}"
            ),
        }
    }
}

impl<E> Error for HistoryEncodeError<E> where E: Error + 'static {}

/// Failed checked load. No authority is returned or replaced.
#[derive(Debug)]
pub enum HistoryLoadError<CE, PE, ME> {
    /// Source bytes exceeded the explicit caller bound.
    EnvelopeTooLarge {
        /// Configured maximum.
        maximum: u64,
        /// Supplied bytes.
        actual: u64,
    },
    /// Source was not valid JSON.
    InvalidJson(serde_json::Error),
    /// The minimum structural header was absent or malformed.
    InvalidHeader(HistoryStructuralHeaderError),
    /// The structural format family is not Longhorn linear history.
    ForeignFormatFamily {
        /// Supplied family.
        actual: String,
    },
    /// The structural version is newer than this implementation.
    FutureStructuralVersion {
        /// Supplied version.
        actual: u32,
        /// Maximum supported version.
        maximum: u32,
    },
    /// No structural step was registered for an older version.
    MissingStructuralMigration {
        /// Unsupported source version.
        from: u32,
    },
    /// A structural hook skipped or mis-stamped one version.
    InvalidStructuralMigration {
        /// Source version.
        from: u32,
        /// Version produced by the hook.
        produced: u32,
    },
    /// A registered structural migration failed.
    StructuralMigration(ME),
    /// Current-version source did not match the strict envelope.
    InvalidEnvelope(serde_json::Error),
    /// The source belongs to a different history authority.
    ForeignHistory {
        /// Expected authority.
        expected: HistoryId,
        /// Supplied authority.
        actual: HistoryId,
    },
    /// The source payload family differs from the registered codec.
    ForeignPayloadCodecFamily {
        /// Registered family.
        expected: HistoryPayloadCodecFamily,
        /// Supplied family.
        actual: HistoryPayloadCodecFamily,
    },
    /// The payload codec version is newer than the registered codec.
    FuturePayloadCodecVersion {
        /// Supplied version.
        actual: HistoryPayloadCodecVersion,
        /// Maximum supported version.
        maximum: HistoryPayloadCodecVersion,
    },
    /// No payload step was registered for one older entry.
    MissingPayloadMigration {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Unsupported source version.
        from: HistoryPayloadCodecVersion,
    },
    /// A payload hook skipped or mis-stamped one version.
    InvalidPayloadMigration {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Source version.
        from: HistoryPayloadCodecVersion,
        /// Version produced by the hook.
        produced: HistoryPayloadCodecVersion,
    },
    /// A registered payload migration failed.
    PayloadMigration {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Consumer codec failure.
        error: CE,
    },
    /// Current payload decoding failed.
    Payload {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Consumer codec failure.
        error: CE,
    },
    /// Decoded payload policy validation failed.
    Policy {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Consumer policy failure.
        error: PE,
    },
    /// Encoded payload length disagreed with source evidence.
    PayloadWeightMismatch {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Source-declared weight.
        recorded: u64,
        /// Exact source byte length.
        actual: u64,
    },
    /// Current codec bytes and consumer policy weight disagreed.
    PolicyWeightMismatch {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Exact codec byte length.
        codec: u64,
        /// Consumer policy measurement.
        policy: u64,
    },
    /// A decoded retained payload is no longer admissible.
    NoOpPayload(HistoryEntryId),
    /// One current payload exceeded the retained weight limit.
    PayloadTooHeavy {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Configured maximum.
        maximum: u64,
        /// Actual current bytes.
        actual: u64,
    },
    /// Total current payload bytes exceeded the retained weight limit.
    RetainedWeightTooLarge {
        /// Configured maximum.
        maximum: u64,
        /// Actual current bytes.
        actual: u64,
    },
    /// Persisted limits were invalid.
    Limits(HistoryLimitsError),
    /// Current position exceeded platform size.
    PositionOverflow,
    /// Current position exceeded retained entries.
    InvalidCurrentPosition {
        /// Retained entries.
        entries: usize,
        /// Supplied position.
        actual: usize,
    },
    /// The next insertion sequence was zero.
    InvalidNextSequence,
    /// Retained-baseline sequence was zero.
    InvalidBaselineSequence,
    /// One retained entry sequence was zero.
    InvalidEntrySequence {
        /// Affected entry.
        entry_id: HistoryEntryId,
    },
    /// One retained label was invalid.
    InvalidLabel {
        /// Affected entry.
        entry_id: HistoryEntryId,
        /// Label validation failure.
        error: crate::HistoryLabelError,
    },
    /// One platform-size or retained-weight conversion overflowed.
    SizeOverflow,
    /// Fully decoded structural state failed current invariants.
    State(HistoryStateError),
}

impl<CE: fmt::Display, PE: fmt::Display, ME: fmt::Display> fmt::Display
    for HistoryLoadError<CE, PE, ME>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EnvelopeTooLarge { maximum, actual } => write!(
                formatter,
                "history envelope is {actual} bytes; configured maximum is {maximum}"
            ),
            Self::InvalidJson(error) => write!(formatter, "invalid history JSON: {error}"),
            Self::InvalidHeader(error) => write!(formatter, "invalid history header: {error}"),
            Self::ForeignFormatFamily { actual } => {
                write!(formatter, "unsupported history format family {actual}")
            }
            Self::FutureStructuralVersion { actual, maximum } => write!(
                formatter,
                "history structural version {actual} is newer than supported version {maximum}"
            ),
            Self::MissingStructuralMigration { from } => {
                write!(
                    formatter,
                    "no history structural migration from version {from}"
                )
            }
            Self::InvalidStructuralMigration { from, produced } => write!(
                formatter,
                "history structural migration from {from} produced invalid version {produced}"
            ),
            Self::StructuralMigration(error) => {
                write!(formatter, "history structural migration failed: {error}")
            }
            Self::InvalidEnvelope(error) => write!(formatter, "invalid history envelope: {error}"),
            Self::ForeignHistory { expected, actual } => write!(
                formatter,
                "history envelope belongs to {actual}; expected {expected}"
            ),
            Self::ForeignPayloadCodecFamily { expected, actual } => write!(
                formatter,
                "history payload codec family is {actual}; expected {expected}"
            ),
            Self::FuturePayloadCodecVersion { actual, maximum } => write!(
                formatter,
                "history payload codec version {} is newer than supported version {}",
                actual.get(),
                maximum.get()
            ),
            Self::MissingPayloadMigration { entry_id, from } => write!(
                formatter,
                "no payload migration for entry {entry_id} from version {}",
                from.get()
            ),
            Self::InvalidPayloadMigration {
                entry_id,
                from,
                produced,
            } => write!(
                formatter,
                "payload migration for entry {entry_id} from {} produced invalid version {}",
                from.get(),
                produced.get()
            ),
            Self::PayloadMigration { entry_id, error } => {
                write!(
                    formatter,
                    "payload migration for entry {entry_id} failed: {error}"
                )
            }
            Self::Payload { entry_id, error } => {
                write!(
                    formatter,
                    "history payload {entry_id} failed to decode: {error}"
                )
            }
            Self::Policy { entry_id, error } => {
                write!(
                    formatter,
                    "history payload {entry_id} failed policy validation: {error}"
                )
            }
            Self::PayloadWeightMismatch {
                entry_id,
                recorded,
                actual,
            } => write!(
                formatter,
                "history payload {entry_id} records weight {recorded}; source has {actual} bytes"
            ),
            Self::PolicyWeightMismatch {
                entry_id,
                codec,
                policy,
            } => write!(
                formatter,
                "history payload {entry_id} has {codec} codec bytes; policy measured {policy}"
            ),
            Self::NoOpPayload(entry_id) => {
                write!(formatter, "history payload {entry_id} is now a no-op")
            }
            Self::PayloadTooHeavy {
                entry_id,
                maximum,
                actual,
            } => write!(
                formatter,
                "history payload {entry_id} weighs {actual}; maximum is {maximum}"
            ),
            Self::RetainedWeightTooLarge { maximum, actual } => write!(
                formatter,
                "history retained payload weight is {actual}; maximum is {maximum}"
            ),
            Self::Limits(error) => write!(formatter, "invalid persisted history limits: {error}"),
            Self::PositionOverflow => {
                formatter.write_str("persisted history current position overflowed")
            }
            Self::InvalidCurrentPosition { entries, actual } => write!(
                formatter,
                "persisted current position {actual} exceeds {entries} entries"
            ),
            Self::InvalidNextSequence => {
                formatter.write_str("persisted history next sequence is zero")
            }
            Self::InvalidBaselineSequence => {
                formatter.write_str("persisted history baseline sequence is zero")
            }
            Self::InvalidEntrySequence { entry_id } => {
                write!(
                    formatter,
                    "persisted history entry {entry_id} has sequence zero"
                )
            }
            Self::InvalidLabel { entry_id, error } => {
                write!(
                    formatter,
                    "persisted history entry {entry_id} label is invalid: {error}"
                )
            }
            Self::SizeOverflow => formatter.write_str("persisted history size overflowed"),
            Self::State(error) => write!(formatter, "persisted history state is invalid: {error}"),
        }
    }
}

impl<CE, PE, ME> Error for HistoryLoadError<CE, PE, ME>
where
    CE: Error + 'static,
    PE: Error + 'static,
    ME: Error + 'static,
{
}

/// Invalid minimal structural header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryStructuralHeaderError {
    /// The document root was not an object.
    NotObject,
    /// The format family was absent or not a string.
    InvalidFamily,
    /// The structural version was absent or not an unsigned 32-bit integer.
    InvalidVersion,
}

impl fmt::Display for HistoryStructuralHeaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotObject => formatter.write_str("history document root must be an object"),
            Self::InvalidFamily => formatter.write_str("history formatFamily must be a string"),
            Self::InvalidVersion => formatter.write_str("history structuralVersion must be a u32"),
        }
    }
}

impl Error for HistoryStructuralHeaderError {}

#[derive(Clone, Debug)]
struct StructuralHeader {
    format_family: String,
    structural_version: u32,
}

fn read_structural_header(
    document: &Value,
) -> Result<StructuralHeader, HistoryStructuralHeaderError> {
    let object = document
        .as_object()
        .ok_or(HistoryStructuralHeaderError::NotObject)?;
    let format_family = object
        .get("formatFamily")
        .and_then(Value::as_str)
        .ok_or(HistoryStructuralHeaderError::InvalidFamily)?
        .to_owned();
    let version = object
        .get("structuralVersion")
        .and_then(Value::as_u64)
        .ok_or(HistoryStructuralHeaderError::InvalidVersion)?;
    let structural_version =
        u32::try_from(version).map_err(|_| HistoryStructuralHeaderError::InvalidVersion)?;
    Ok(StructuralHeader {
        format_family,
        structural_version,
    })
}

fn validate_envelope_size(length: usize, maximum: u64) -> Result<(), (u64, u64)> {
    let actual = u64::try_from(length).map_err(|_| (maximum, u64::MAX))?;
    if actual > maximum {
        Err((maximum, actual))
    } else {
        Ok(())
    }
}

fn decode_limits(limits: PersistedHistoryLimits) -> Result<HistoryLimits, HistoryLimitsError> {
    let maximum_entries = usize::try_from(limits.maximum_entries).map_err(|_| {
        HistoryLimitsError::TooManyEntries {
            maximum: crate::limits::MAXIMUM_HISTORY_ENTRIES,
            actual: usize::MAX,
        }
    })?;
    let maximum_label_bytes = usize::try_from(limits.maximum_label_bytes).map_err(|_| {
        HistoryLimitsError::LabelBytesTooLarge {
            maximum: crate::MAXIMUM_HISTORY_LABEL_BYTES,
            actual: usize::MAX,
        }
    })?;
    HistoryLimits::new(
        maximum_entries,
        limits.maximum_encoded_weight,
        maximum_label_bytes,
    )
}

fn decode_baseline(
    baseline: PersistedRetainedBaseline,
) -> Result<HistoryRetainedBaseline, crate::HistoryEntrySequenceZero> {
    let last_pruned_sequence = baseline
        .last_pruned_sequence
        .map(HistoryEntrySequence::new)
        .transpose()?;
    Ok(HistoryRetainedBaseline::new(
        baseline.pruned_entry_count,
        baseline.pruned_encoded_weight,
        baseline.last_pruned_entry_id,
        last_pruned_sequence,
    ))
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PersistedLinearHistoryEnvelope {
    format_family: String,
    structural_version: u32,
    payload_codec: PersistedPayloadCodec,
    mode: PersistedHistoryMode,
    history_id: HistoryId,
    revision: HistoryRevision,
    limits: PersistedHistoryLimits,
    next_sequence: u64,
    retained_baseline: PersistedRetainedBaseline,
    current_position: u64,
    entries: Vec<PersistedHistoryEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PersistedPayloadCodec {
    family: HistoryPayloadCodecFamily,
    version: HistoryPayloadCodecVersion,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum PersistedHistoryMode {
    Linear,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PersistedHistoryLimits {
    maximum_entries: u64,
    maximum_encoded_weight: u64,
    maximum_label_bytes: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PersistedRetainedBaseline {
    pruned_entry_count: u64,
    pruned_encoded_weight: u64,
    last_pruned_entry_id: Option<HistoryEntryId>,
    last_pruned_sequence: Option<u64>,
}

impl From<&HistoryRetainedBaseline> for PersistedRetainedBaseline {
    fn from(value: &HistoryRetainedBaseline) -> Self {
        Self {
            pruned_entry_count: value.pruned_entry_count(),
            pruned_encoded_weight: value.pruned_encoded_weight(),
            last_pruned_entry_id: value.last_pruned_entry_id().cloned(),
            last_pruned_sequence: value.last_pruned_sequence().map(HistoryEntrySequence::get),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PersistedHistoryEntry {
    entry_id: HistoryEntryId,
    label: String,
    kind_id: Option<HistoryKindId>,
    group_id: Option<HistoryGroupId>,
    sequence: u64,
    committed_revision: HistoryRevision,
    encoded_weight: u64,
    payload: Vec<u8>,
}
