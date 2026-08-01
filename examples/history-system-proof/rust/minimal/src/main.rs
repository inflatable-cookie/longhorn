use std::convert::Infallible;

use longhorn_core::{HistoryEntryId, HistoryId, HistoryKindId, HistoryPlanId, HistoryRevision};
use longhorn_history::{
    AppliedHistoryRecord, HistoryAuthorityEpoch, HistoryChangedEvent, HistoryCoalesce,
    HistoryCoalesceContext, HistoryEntryMetadata, HistoryLabel, HistoryLimits,
    HistoryNavigationPlan, HistoryNavigationReceiptProjection, HistoryNavigationRequest,
    HistoryNavigationResult, HistoryNavigationTarget, HistoryNavigationTransaction,
    HistoryNavigationTransactionFailure, HistoryPageRequest, HistoryPageSnapshot, HistoryPolicy,
    HistorySnapshot, LinearHistory,
};
use serde_json::{Value, json};

#[derive(Clone, Debug, Eq, PartialEq)]
struct PreferenceMutation {
    key: &'static str,
    before: i32,
    after: i32,
}

struct PreferencePolicy;

impl HistoryPolicy<PreferenceMutation> for PreferencePolicy {
    type Error = Infallible;

    fn inverse(&self, payload: &PreferenceMutation) -> Result<PreferenceMutation, Self::Error> {
        Ok(PreferenceMutation {
            key: payload.key,
            before: payload.after,
            after: payload.before,
        })
    }

    fn is_noop(&self, payload: &PreferenceMutation) -> bool {
        payload.before == payload.after
    }

    fn encoded_weight(&self, payload: &PreferenceMutation) -> Result<u64, Self::Error> {
        Ok(u64::try_from(payload.key.len()).unwrap() + 8)
    }

    fn coalesce(
        &self,
        previous: &PreferenceMutation,
        incoming: &PreferenceMutation,
        _: HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<PreferenceMutation>, Self::Error> {
        if previous.key == incoming.key {
            Ok(HistoryCoalesce::Replace(PreferenceMutation {
                key: previous.key,
                before: previous.before,
                after: incoming.after,
            }))
        } else {
            Ok(HistoryCoalesce::KeepSeparate)
        }
    }
}

struct AcceptTransaction;

impl HistoryNavigationTransaction<PreferenceMutation> for AcceptTransaction {
    type Error = Infallible;

    fn apply(
        &mut self,
        _: &HistoryNavigationPlan<PreferenceMutation>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        Ok(())
    }
}

fn main() {
    let policy = PreferencePolicy;
    let mut history = LinearHistory::new(
        HistoryId::new("history:minimal-preferences").unwrap(),
        HistoryLimits::default(),
    );
    record(
        &mut history,
        0,
        "entry:font-1",
        "Set font size",
        PreferenceMutation {
            key: "font-size",
            before: 12,
            after: 13,
        },
        &policy,
    );
    record(
        &mut history,
        1,
        "entry:font-2",
        "Set font size to 14",
        PreferenceMutation {
            key: "font-size",
            before: 13,
            after: 14,
        },
        &policy,
    );
    assert_eq!(history.applied().len(), 1);
    record(
        &mut history,
        2,
        "entry:zoom",
        "Set zoom",
        PreferenceMutation {
            key: "zoom",
            before: 100,
            after: 125,
        },
        &policy,
    );
    record(
        &mut history,
        3,
        "entry:density",
        "Set density",
        PreferenceMutation {
            key: "density",
            before: 1,
            after: 2,
        },
        &policy,
    );

    navigate(
        &mut history,
        "plan:seed-future",
        HistoryNavigationTarget::Undo,
        &policy,
    );
    let epoch = HistoryAuthorityEpoch::new(1).unwrap();
    let initial_snapshot = snapshot(epoch, &history);
    let initial_page = page(epoch, &history);
    assert_eq!(initial_snapshot.summary.redo_depth, 1);

    let plan = history
        .plan_navigation(
            HistoryNavigationRequest::new(
                HistoryPlanId::new("plan:renderer-expected").unwrap(),
                history.revision(),
                HistoryNavigationTarget::Undo,
            ),
            &policy,
        )
        .unwrap();
    let receipt = history
        .execute_navigation(plan, &mut AcceptTransaction)
        .unwrap();
    let changed_event = HistoryChangedEvent::from_transition(epoch, receipt.transition());
    let committed_snapshot = snapshot(epoch, &history);
    let committed_page = page(epoch, &history);
    let navigation_result = HistoryNavigationResult::Committed {
        snapshot: committed_snapshot.clone(),
        receipt: Box::new(HistoryNavigationReceiptProjection::from_receipt(&receipt).unwrap()),
    };
    let expected_public_trace = public_trace(&committed_snapshot, &committed_page);

    println!(
        "{}",
        json!({
            "shape": "minimal",
            "mechanics": {
                "record": true,
                "coalesce": true,
                "undo": true,
                "authoritativeFuture": true
            },
            "publicTrace": expected_public_trace,
            "rendererFixture": {
                "initialSnapshot": initial_snapshot,
                "initialPage": initial_page,
                "navigationResult": navigation_result,
                "committedPage": committed_page,
                "changedEvent": changed_event,
                "expectedPublicTrace": expected_public_trace
            }
        })
    );
}

fn record(
    history: &mut LinearHistory<PreferenceMutation>,
    revision: u64,
    entry: &str,
    label: &str,
    payload: PreferenceMutation,
    policy: &PreferencePolicy,
) {
    history
        .record_applied(
            AppliedHistoryRecord::new(
                HistoryRevision::new(revision),
                HistoryEntryId::new(entry).unwrap(),
                HistoryEntryMetadata::new(
                    HistoryLabel::new(label).unwrap(),
                    Some(HistoryKindId::new("preference:set").unwrap()),
                    None,
                ),
                payload,
            ),
            policy,
        )
        .unwrap();
}

fn navigate(
    history: &mut LinearHistory<PreferenceMutation>,
    plan_id: &str,
    target: HistoryNavigationTarget,
    policy: &PreferencePolicy,
) {
    let plan = history
        .plan_navigation(
            HistoryNavigationRequest::new(
                HistoryPlanId::new(plan_id).unwrap(),
                history.revision(),
                target,
            ),
            policy,
        )
        .unwrap();
    history
        .execute_navigation(plan, &mut AcceptTransaction)
        .unwrap();
}

fn snapshot(
    epoch: HistoryAuthorityEpoch,
    history: &LinearHistory<PreferenceMutation>,
) -> HistorySnapshot {
    HistorySnapshot::from_summary(epoch, &history.project_summary().unwrap()).unwrap()
}

fn page(
    epoch: HistoryAuthorityEpoch,
    history: &LinearHistory<PreferenceMutation>,
) -> HistoryPageSnapshot {
    HistoryPageSnapshot::from_page(
        epoch,
        &history
            .project_page(HistoryPageRequest::new(0, 50).unwrap())
            .unwrap(),
    )
    .unwrap()
}

fn public_trace(snapshot: &HistorySnapshot, page: &HistoryPageSnapshot) -> Value {
    json!({
        "revision": snapshot.summary.revision.get(),
        "undoDepth": snapshot.summary.undo_depth,
        "redoDepth": snapshot.summary.redo_depth,
        "currentEntryId": snapshot
            .summary
            .current_entry_id
            .as_ref()
            .map(HistoryEntryId::as_str),
        "entries": page.entries.iter().map(|entry| json!({
            "entryId": entry.entry_id.as_str(),
            "position": match entry.position {
                longhorn_history::HistoryProjectionPosition::Past => "past",
                longhorn_history::HistoryProjectionPosition::Current => "current",
                longhorn_history::HistoryProjectionPosition::Future => "future",
            }
        })).collect::<Vec<_>>()
    })
}
