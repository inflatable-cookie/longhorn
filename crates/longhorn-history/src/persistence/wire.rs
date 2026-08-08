//! Private persisted envelope shape.

use longhorn_core::{HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryRevision};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{HistoryEntrySequence, HistoryLimits, HistoryLimitsError, HistoryRetainedBaseline};

use super::{HistoryPayloadCodecFamily, HistoryPayloadCodecVersion, HistoryStructuralHeaderError};

#[derive(Clone, Debug)]
pub(super) struct StructuralHeader {
    pub(super) format_family: String,
    pub(super) structural_version: u32,
}

pub(super) fn read_structural_header(
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

pub(super) fn validate_envelope_size(length: usize, maximum: u64) -> Result<(), (u64, u64)> {
    let actual = u64::try_from(length).map_err(|_| (maximum, u64::MAX))?;
    if actual > maximum {
        Err((maximum, actual))
    } else {
        Ok(())
    }
}

pub(super) fn decode_limits(
    limits: PersistedHistoryLimits,
) -> Result<HistoryLimits, HistoryLimitsError> {
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

pub(super) fn decode_baseline(
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
pub(super) struct PersistedLinearHistoryEnvelope {
    pub(super) format_family: String,
    pub(super) structural_version: u32,
    pub(super) payload_codec: PersistedPayloadCodec,
    pub(super) mode: PersistedHistoryMode,
    pub(super) history_id: HistoryId,
    pub(super) revision: HistoryRevision,
    pub(super) limits: PersistedHistoryLimits,
    pub(super) next_sequence: u64,
    pub(super) retained_baseline: PersistedRetainedBaseline,
    pub(super) current_position: u64,
    pub(super) entries: Vec<PersistedHistoryEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct PersistedPayloadCodec {
    pub(super) family: HistoryPayloadCodecFamily,
    pub(super) version: HistoryPayloadCodecVersion,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum PersistedHistoryMode {
    Linear,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct PersistedHistoryLimits {
    pub(super) maximum_entries: u64,
    pub(super) maximum_encoded_weight: u64,
    pub(super) maximum_label_bytes: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct PersistedRetainedBaseline {
    pub(super) pruned_entry_count: u64,
    pub(super) pruned_encoded_weight: u64,
    pub(super) last_pruned_entry_id: Option<HistoryEntryId>,
    pub(super) last_pruned_sequence: Option<u64>,
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
pub(super) struct PersistedHistoryEntry {
    pub(super) entry_id: HistoryEntryId,
    pub(super) label: String,
    pub(super) kind_id: Option<HistoryKindId>,
    pub(super) group_id: Option<HistoryGroupId>,
    pub(super) sequence: u64,
    pub(super) committed_revision: HistoryRevision,
    pub(super) encoded_weight: u64,
    pub(super) payload: Vec<u8>,
}
