use longhorn_core::{HistoryEntryId, HistoryId, HistoryRevision};
use longhorn_history::HistoryAuthorityEpoch;
use serde::{Deserialize, Serialize};

use crate::{ForkBranchId, ForkBranchPage};

use super::{ForkHistoryProtocolVersion, ForkProtocolProjectionError, count};

/// One payload-free first-class branch record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkBranchRecord {
    /// Stable branch identity.
    pub branch_id: ForkBranchId,
    /// Branch head, or root.
    pub head_entry_id: Option<HistoryEntryId>,
    /// Last entry shared with this branch's nearest ancestor branch -- where
    /// it forked. Relative to the parent run, never to the current branch.
    pub divergence_entry_id: Option<HistoryEntryId>,
    /// The branch it forked off, paired with the entry above.
    pub divergence_branch_id: Option<ForkBranchId>,
    /// Optional branch name.
    pub name: Option<String>,
    /// Optional branch annotation.
    pub annotation: Option<String>,
    /// Retention pin state.
    pub pinned: bool,
    /// Whether this is the selected branch.
    pub current: bool,
}

/// Revision-bound request for one bounded branch page.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkBranchPageCommand {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Authority lifetime observed by caller.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// History identity observed by caller.
    pub history_id: HistoryId,
    /// Exact graph revision required.
    pub expected_revision: HistoryRevision,
    /// Stable-id ordered offset.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub offset: u64,
    /// Maximum requested records.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub limit: u64,
}

/// One bounded authoritative first-class branch page.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkBranchPageSnapshot {
    /// Exact metadata protocol line.
    pub protocol_version: ForkHistoryProtocolVersion,
    /// Live authority lifetime.
    pub authority_epoch: HistoryAuthorityEpoch,
    /// Stable graph identity.
    pub history_id: HistoryId,
    /// Exact projected revision.
    pub revision: HistoryRevision,
    /// Stable-id ordered offset.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub offset: u64,
    /// Total first-class branch count.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub total_branches: u64,
    /// Bounded branch metadata.
    pub branches: Vec<ForkBranchRecord>,
    /// Whether earlier stable ids precede this page.
    pub truncated_before: bool,
    /// Whether later stable ids follow this page.
    pub truncated_after: bool,
}

impl ForkBranchPageSnapshot {
    /// Projects one checked pure branch page.
    pub fn from_page(
        authority_epoch: HistoryAuthorityEpoch,
        page: &ForkBranchPage,
    ) -> Result<Self, ForkProtocolProjectionError> {
        Ok(Self {
            protocol_version: ForkHistoryProtocolVersion::CURRENT,
            authority_epoch,
            history_id: page.history_id().clone(),
            revision: page.revision(),
            offset: count(page.offset())?,
            total_branches: count(page.total_branches())?,
            branches: page
                .branches()
                .iter()
                .map(|branch| ForkBranchRecord {
                    branch_id: branch.branch_id().clone(),
                    head_entry_id: branch.head_entry_id().cloned(),
                    divergence_entry_id: branch.divergence_entry_id().cloned(),
                    divergence_branch_id: branch.divergence_branch_id().cloned(),
                    name: branch.name().map(str::to_owned),
                    annotation: branch.annotation().map(str::to_owned),
                    pinned: branch.pinned(),
                    current: branch.current(),
                })
                .collect(),
            truncated_before: page.truncated_before(),
            truncated_after: page.truncated_after(),
        })
    }
}
