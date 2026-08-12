//! Load and validate persisted linear-history envelopes.

use longhorn_core::HistoryId;
use serde_json::Value;

use crate::{
    HistoryCommittedTransition, HistoryCommittedTransitionKind, HistoryEntry, HistoryEntryMetadata,
    HistoryEntrySequence, HistoryLabel, HistoryNavigationLimits, HistoryPolicy,
    HistoryProjectionLimits, LinearHistory, LinearHistoryState, MAXIMUM_HISTORY_ENCODED_WEIGHT,
};

use super::wire::{
    PersistedLinearHistoryEnvelope, decode_baseline, decode_limits, read_structural_header,
    validate_envelope_size,
};
use super::{
    CURRENT_HISTORY_STRUCTURAL_VERSION, HISTORY_FORMAT_FAMILY, HistoryLoadAttempt,
    HistoryLoadError, HistoryLoadOutcome, HistoryLoadReceipt, HistoryLoadResult,
    HistoryPayloadCodec, HistoryPayloadMigrationTarget, HistoryPersistence,
    HistoryStructuralHeaderError, HistoryStructuralMigration, HistoryStructuralMigrationTarget,
};

impl<C, M> HistoryPersistence<C, M>
where
    M: HistoryStructuralMigration,
{
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
                match entry.recorded_at {
                    Some(recorded_at) => {
                        HistoryEntryMetadata::new(label, entry.kind_id, entry.group_id)
                            .with_recorded_at(recorded_at)
                    }
                    None => HistoryEntryMetadata::new(label, entry.kind_id, entry.group_id),
                },
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
