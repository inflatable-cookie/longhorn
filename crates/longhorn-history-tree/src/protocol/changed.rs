use longhorn_core::{HistoryId, HistoryRevision};
use longhorn_history::HistoryAuthorityEpoch;
use serde::{Deserialize, Serialize};

use super::ForkHistoryProtocolVersion;

/// Coarse non-durable committed graph transition kind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum ForkChangedKind {
    /// A product mutation recorded a node.
    Record,
    /// Checked graph navigation committed.
    Navigation,
    /// Branch metadata changed.
    BranchMetadata,
    /// Retention pruned graph authority.
    Retention,
    /// Checkpoint metadata changed.
    Checkpoint,
    /// Persisted graph authority loaded.
    Imported,
    /// Graph authority reset.
    Reset,
}

/// Non-durable live invalidation hint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkChangedEvent {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Live authority lifetime.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// Stable graph identity.
    pub history_id: HistoryId,
    /// Previous graph revision, absent for load recovery.
    pub previous_revision: Option<HistoryRevision>,
    /// Authoritative resulting revision.
    pub committed_revision: HistoryRevision,
    /// Coarse invalidation category.
    pub kind: ForkChangedKind,
}

/// A platform-sized projection count exceeded the fixed protocol type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForkProtocolProjectionError;

impl std::fmt::Display for ForkProtocolProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("fork projection count exceeds protocol bound")
    }
}

impl std::error::Error for ForkProtocolProjectionError {}

pub(crate) fn count(value: usize) -> Result<u64, ForkProtocolProjectionError> {
    u64::try_from(value).map_err(|_| ForkProtocolProjectionError)
}
