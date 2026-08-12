//! Encode persisted linear-history envelopes.

use crate::LinearHistory;

use super::wire::{
    PersistedHistoryEntry, PersistedHistoryLimits, PersistedHistoryMode,
    PersistedLinearHistoryEnvelope, PersistedPayloadCodec, PersistedRetainedBaseline,
    validate_envelope_size,
};
use super::{
    CURRENT_HISTORY_STRUCTURAL_VERSION, HISTORY_FORMAT_FAMILY, HistoryEncodeError,
    HistoryPayloadCodec, HistoryPersistence, HistoryStructuralMigration,
};

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
                recorded_at: entry.metadata().recorded_at(),
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
}
