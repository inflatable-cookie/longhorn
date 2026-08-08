//! JSON wire shapes for fork-history envelopes.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use longhorn_core::{
    HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryRevision,
};
use longhorn_history::{HistoryPayloadCodecFamily, HistoryPayloadCodecVersion};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{ForkBranchId, ForkCheckpointId};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Header {
    pub(crate) format_family: String,
    pub(crate) structural_version: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Envelope {
    pub(crate) format_family: String,
    pub(crate) structural_version: u32,
    pub(crate) payload_codec: PayloadCodec,
    pub(crate) history_id: HistoryId,
    pub(crate) revision: HistoryRevision,
    pub(crate) current_branch_id: ForkBranchId,
    pub(crate) current_node_id: Option<HistoryEntryId>,
    pub(crate) next_sequence: u64,
    pub(crate) nodes: Vec<Node>,
    pub(crate) branches: Vec<Branch>,
    pub(crate) preferred_children: Vec<PreferredChild>,
    pub(crate) checkpoints: Vec<Checkpoint>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct PayloadCodec {
    pub(crate) family: HistoryPayloadCodecFamily,
    pub(crate) version: HistoryPayloadCodecVersion,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Node {
    pub(crate) entry_id: HistoryEntryId,
    pub(crate) parent_entry_id: Option<HistoryEntryId>,
    pub(crate) label: String,
    pub(crate) kind_id: Option<HistoryKindId>,
    pub(crate) group_id: Option<HistoryGroupId>,
    pub(crate) sequence: u64,
    pub(crate) committed_revision: HistoryRevision,
    pub(crate) encoded_weight: u64,
    #[serde(with = "base64_bytes")]
    pub(crate) payload: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Branch {
    pub(crate) branch_id: ForkBranchId,
    pub(crate) head_entry_id: Option<HistoryEntryId>,
    pub(crate) name: Option<String>,
    pub(crate) annotation: Option<String>,
    pub(crate) pinned: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct PreferredChild {
    pub(crate) parent_entry_id: Option<HistoryEntryId>,
    pub(crate) child_entry_id: HistoryEntryId,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Checkpoint {
    pub(crate) checkpoint_id: ForkCheckpointId,
    pub(crate) after_entry_id: Option<HistoryEntryId>,
    pub(crate) consumer_reference: String,
}

mod base64_bytes {
    use super::*;

    pub(super) fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        STANDARD.decode(value).map_err(de::Error::custom)
    }
}
