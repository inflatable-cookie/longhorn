use std::error::Error;

use longhorn_core::{
    HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryPlanId, HistoryRevision,
};
use longhorn_history::{
    HistoryAuthorityEpoch, HistoryBaselineProjection, HistoryChangedEvent, HistoryChangedKind,
    HistoryEntryRecord, HistoryNavigationCommand, HistoryNavigationDirectionProjection,
    HistoryNavigationPositionProjection, HistoryNavigationReceiptProjection,
    HistoryNavigationRejectionCode, HistoryNavigationRejectionProjection, HistoryNavigationResult,
    HistoryNavigationTargetProjection, HistoryPageCommand, HistoryPageSnapshot,
    HistoryProjectionPosition, HistoryProtocolMode, HistoryProtocolVersion, HistoryRecordedAt,
    HistorySnapshot, HistorySummaryProjection,
};
use serde_json::{json, to_value};

pub fn render() -> Result<String, Box<dyn Error>> {
    let epoch = HistoryAuthorityEpoch::new(7)?;
    let history_id = id::<HistoryId>("history:fixture");
    let current_entry_id = id::<HistoryEntryId>("entry:current");
    let future_entry_id = id::<HistoryEntryId>("entry:future");
    let baseline = HistoryBaselineProjection {
        pruned_entry_count: 2,
        pruned_encoded_weight: 48,
        last_pruned_entry_id: Some(id::<HistoryEntryId>("entry:pruned")),
        last_pruned_sequence: Some(2),
    };
    let snapshot = HistorySnapshot {
        protocol_version: HistoryProtocolVersion::CURRENT,
        authority_epoch: epoch,
        summary: HistorySummaryProjection {
            history_id: history_id.clone(),
            revision: HistoryRevision::new(11),
            mode: HistoryProtocolMode::Linear,
            undo_depth: 2,
            redo_depth: 1,
            current_entry_id: Some(current_entry_id.clone()),
            next_undo_label: Some("Move clip".into()),
            next_redo_label: Some("Rename track".into()),
            retained_entry_count: 3,
            retained_encoded_weight: 96,
            retained_baseline: baseline.clone(),
        },
    };
    let page = HistoryPageSnapshot {
        protocol_version: HistoryProtocolVersion::CURRENT,
        authority_epoch: epoch,
        history_id: history_id.clone(),
        revision: HistoryRevision::new(11),
        offset: 0,
        total_entries: 3,
        entries: vec![
            HistoryEntryRecord {
                entry_id: future_entry_id.clone(),
                label: "Rename track".into(),
                kind_id: Some(id::<HistoryKindId>("track:rename")),
                group_id: None,
                // A host that keeps no clock leaves the stamp absent.
                recorded_at: None,
                sequence: 5,
                committed_revision: HistoryRevision::new(9),
                encoded_weight: 32,
                position: HistoryProjectionPosition::Future,
            },
            HistoryEntryRecord {
                entry_id: current_entry_id.clone(),
                label: "Move clip".into(),
                kind_id: Some(id::<HistoryKindId>("clip:move")),
                group_id: Some(id::<HistoryGroupId>("gesture:move")),
                recorded_at: Some(HistoryRecordedAt::from_epoch_millis(1_765_432_100_000)),
                sequence: 4,
                committed_revision: HistoryRevision::new(11),
                encoded_weight: 40,
                position: HistoryProjectionPosition::Current,
            },
        ],
        truncated_before: false,
        truncated_after: true,
        retained_baseline: baseline,
    };
    let command = HistoryNavigationCommand {
        protocol_version: HistoryProtocolVersion::CURRENT,
        authority_epoch: epoch,
        history_id: history_id.clone(),
        plan_id: id::<HistoryPlanId>("plan:undo"),
        expected_revision: HistoryRevision::new(11),
        target: HistoryNavigationTargetProjection::Undo,
    };
    let position = HistoryNavigationPositionProjection {
        applied_depth: 2,
        future_depth: 1,
        current_entry_id: Some(current_entry_id.clone()),
        next_undo_label: Some("Move clip".into()),
        next_redo_entry_id: Some(future_entry_id.clone()),
        next_redo_label: Some("Rename track".into()),
    };
    let receipt = HistoryNavigationReceiptProjection {
        history_id: history_id.clone(),
        plan_id: id::<HistoryPlanId>("plan:undo"),
        previous_revision: HistoryRevision::new(11),
        committed_revision: HistoryRevision::new(12),
        direction: HistoryNavigationDirectionProjection::Undo,
        moved_entry_ids: vec![current_entry_id.clone()],
        source_position: position.clone(),
        authoritative_position: HistoryNavigationPositionProjection {
            applied_depth: 1,
            future_depth: 2,
            current_entry_id: Some(id::<HistoryEntryId>("entry:past")),
            next_undo_label: Some("Create clip".into()),
            next_redo_entry_id: Some(current_entry_id.clone()),
            next_redo_label: Some("Move clip".into()),
        },
    };
    let committed_snapshot = HistorySnapshot {
        summary: HistorySummaryProjection {
            revision: HistoryRevision::new(12),
            undo_depth: 1,
            redo_depth: 2,
            current_entry_id: Some(id::<HistoryEntryId>("entry:past")),
            next_undo_label: Some("Create clip".into()),
            next_redo_label: Some("Move clip".into()),
            ..snapshot.summary.clone()
        },
        ..snapshot.clone()
    };
    let rejection = HistoryNavigationRejectionProjection {
        code: HistoryNavigationRejectionCode::StaleRevision,
        detail: "history changed before navigation".into(),
        refresh_required: true,
    };
    let fixture = json!({
        "protocolVersion": 1,
        "snapshot": to_value(&snapshot)?,
        "pageRequest": to_value(HistoryPageCommand {
            protocol_version: HistoryProtocolVersion::CURRENT,
            authority_epoch: epoch,
            history_id: history_id.clone(),
            expected_revision: HistoryRevision::new(11),
            offset: 0,
            limit: 2,
        })?,
        "page": to_value(page)?,
        "navigationCommand": to_value(command)?,
        "navigationResults": [
            to_value(HistoryNavigationResult::Committed {
                snapshot: committed_snapshot.clone(),
                receipt: Box::new(receipt),
            })?,
            to_value(HistoryNavigationResult::Rejected {
                snapshot: committed_snapshot.clone(),
                rejection,
            })?,
        ],
        "changedEvent": to_value(HistoryChangedEvent {
            protocol_version: HistoryProtocolVersion::CURRENT,
            authority_epoch: epoch,
            history_id,
            previous_revision: Some(HistoryRevision::new(11)),
            committed_revision: HistoryRevision::new(12),
            kind: HistoryChangedKind::Navigation,
        })?,
        "incompatibility": {
            "futureProtocolVersion": 2,
            "unknownMode": "forked",
            "unknownEntryPosition": "alternate",
            "unknownNavigationTarget": "branch",
            "unknownNavigationDirection": "branch",
            "unknownRejectionCode": "futureRejection",
            "unknownNavigationStatus": "futureStatus",
            "unknownChangedKind": "futureKind"
        }
    });
    Ok(format!("{}\n", serde_json::to_string_pretty(&fixture)?))
}

fn id<T>(value: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    value.parse().expect("fixture id must be valid")
}
