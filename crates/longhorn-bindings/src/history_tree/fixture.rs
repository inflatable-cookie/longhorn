use std::error::Error;

use longhorn_core::{HistoryEntryId, HistoryId, HistoryRevision};
use longhorn_history::HistoryAuthorityEpoch;
use longhorn_history_tree::{
    ForkBranchId, ForkHistoryProtocolVersion, ForkSnapshot, ForkSummaryProjection,
};

pub(super) fn render() -> Result<String, Box<dyn Error>> {
    let snapshot = ForkSnapshot {
        protocol_version: ForkHistoryProtocolVersion::CURRENT,
        authority_epoch: HistoryAuthorityEpoch::new(7)?,
        summary: ForkSummaryProjection {
            history_id: HistoryId::new("history:fixture")?,
            revision: HistoryRevision::new(12),
            current_branch_id: ForkBranchId::new("branch:main")?,
            current_entry_id: Some(HistoryEntryId::new("entry:current")?),
            undo_depth: 3,
            redo_depth: 1,
            next_undo_label: Some("Move panel".to_owned()),
            next_redo_label: Some("Resize region".to_owned()),
            retained_entry_count: 5,
            retained_encoded_weight: 640,
            branch_count: 2,
            alternate_path_count: 2,
        },
    };
    let rejections = retention_rejections()?;
    let fixture = serde_json::json!({ "snapshot": snapshot, "retentionRejections": rejections });
    Ok(format!("{}\n", serde_json::to_string_pretty(&fixture)?))
}

/// Wire examples generated from the same Rust projections consumers receive.
fn retention_rejections() -> Result<serde_json::Value, Box<dyn Error>> {
    use longhorn_history_tree::{
        ForkDeleteResult, ForkPruneResult, ForkRetentionError, ForkRetentionRejectionProjection,
    };
    let projection = |error| {
        ForkRetentionRejectionProjection::from_retention_error(&error)
            .expect("expected retention refusal")
    };
    let values = serde_json::json!({
        "deletePinned": ForkDeleteResult::Rejected { rejection: projection(ForkRetentionError::PinnedBranchInSubtree(ForkBranchId::new("branch:alternate")?)) },
        "deleteCurrent": ForkDeleteResult::Rejected { rejection: projection(ForkRetentionError::CurrentNodeInSubtree(HistoryEntryId::new("entry:current")?)) },
        "pruneStale": ForkPruneResult::Rejected { rejection: projection(ForkRetentionError::StaleRevision { expected: HistoryRevision::new(1), actual: HistoryRevision::new(2) }) },
        "pruneZero": ForkPruneResult::Rejected { rejection: projection(ForkRetentionError::ZeroLimit) },
    });
    Ok(values)
}
