use std::convert::Infallible;

use longhorn_core::{HistoryPlanId, HistoryRevision};
use longhorn_history::{
    HistoryAuthorityEpoch, HistoryChangedEvent, HistoryChangedKind, HistoryLimits,
    HistoryNavigationPlan, HistoryNavigationReceiptProjection, HistoryNavigationRequest,
    HistoryNavigationTarget, HistoryNavigationTransaction, HistoryNavigationTransactionFailure,
    HistoryPageRequest, HistoryPageSnapshot, HistorySnapshot, LinearHistory,
};

use crate::{
    pulse_shaped::{PulseFixtureMutation, PulseFixturePolicy, rename},
    support::*,
};

struct AcceptTransaction;

impl HistoryNavigationTransaction<PulseFixtureMutation> for AcceptTransaction {
    type Error = Infallible;

    fn apply(
        &mut self,
        _plan: &HistoryNavigationPlan<PulseFixtureMutation>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        Ok(())
    }
}

#[test]
fn kernel_summary_page_receipt_and_event_project_without_product_payload() {
    let epoch = HistoryAuthorityEpoch::new(3).unwrap();
    let mut history = LinearHistory::new(history_id("history:protocol"), HistoryLimits::default());
    history
        .record_applied(
            record(
                0,
                "entry:rename",
                metadata("Rename track", "track:rename"),
                rename(1, "Bass", "Low"),
            ),
            &PulseFixturePolicy,
        )
        .unwrap();

    let snapshot =
        HistorySnapshot::from_summary(epoch, &history.project_summary().unwrap()).unwrap();
    let page = HistoryPageSnapshot::from_page(
        epoch,
        &history
            .project_page(HistoryPageRequest::new(0, 20).unwrap())
            .unwrap(),
    )
    .unwrap();
    let plan = history
        .plan_navigation(
            HistoryNavigationRequest::new(
                HistoryPlanId::new("plan:protocol").unwrap(),
                HistoryRevision::new(1),
                HistoryNavigationTarget::Undo,
            ),
            &PulseFixturePolicy,
        )
        .unwrap();
    let receipt = history
        .execute_navigation(plan, &mut AcceptTransaction)
        .unwrap();
    let projected_receipt = HistoryNavigationReceiptProjection::from_receipt(&receipt).unwrap();
    let event = HistoryChangedEvent::from_transition(epoch, receipt.transition());

    assert_eq!(
        snapshot.summary.next_undo_label.as_deref(),
        Some("Rename track")
    );
    assert_eq!(page.entries[0].label, "Rename track");
    assert_eq!(
        projected_receipt.moved_entry_ids,
        [entry_id("entry:rename")]
    );
    assert_eq!(event.kind, HistoryChangedKind::Navigation);
    assert_eq!(event.committed_revision.get(), 2);

    let serialized = serde_json::to_value((snapshot, page, projected_receipt, event)).unwrap();
    assert!(!contains_key(&serialized, "payload"));
}

#[test]
fn zero_authority_epoch_rejects_at_construction_and_deserialization() {
    assert!(HistoryAuthorityEpoch::new(0).is_err());
    assert!(serde_json::from_str::<HistoryAuthorityEpoch>("0").is_err());
}

fn contains_key(value: &serde_json::Value, key: &str) -> bool {
    match value {
        serde_json::Value::Object(values) => {
            values.contains_key(key) || values.values().any(|value| contains_key(value, key))
        }
        serde_json::Value::Array(values) => values.iter().any(|value| contains_key(value, key)),
        _ => false,
    }
}

#[test]
fn history_tagged_wire_enums_reject_unknown_fields() {
    use longhorn_history::{
        HistoryNavigationRejectionCode, HistoryNavigationRejectionProjection,
        HistoryNavigationResult, HistoryNavigationTargetProjection, HistoryPageFloorProjection,
    };
    use serde_json::{Value, json};

    // Contract 010: unknown fields are rejected on every wire type, tagged
    // variants included.
    let epoch = HistoryAuthorityEpoch::new(3).unwrap();
    let history = LinearHistory::<PulseFixtureMutation>::new(
        history_id("history:strict"),
        HistoryLimits::default(),
    );
    let snapshot = HistorySnapshot::from_summary(epoch, &history.project_summary().unwrap())
        .unwrap();
    let with_unknown_field = |mut value: Value| {
        value["unexpected"] = json!(true);
        value
    };

    let undo = serde_json::to_value(HistoryNavigationTargetProjection::Undo {}).unwrap();
    assert!(
        serde_json::from_value::<HistoryNavigationTargetProjection>(undo.clone()).is_ok()
    );
    assert!(
        serde_json::from_value::<HistoryNavigationTargetProjection>(with_unknown_field(undo))
            .is_err()
    );
    let checkout = json!({"kind": "checkout", "entryId": "entry:rename"});
    assert!(
        serde_json::from_value::<HistoryNavigationTargetProjection>(checkout.clone()).is_ok()
    );
    assert!(
        serde_json::from_value::<HistoryNavigationTargetProjection>(with_unknown_field(checkout))
            .is_err()
    );

    let origin = serde_json::to_value(HistoryPageFloorProjection::Origin {}).unwrap();
    assert!(serde_json::from_value::<HistoryPageFloorProjection>(origin.clone()).is_ok());
    assert!(
        serde_json::from_value::<HistoryPageFloorProjection>(with_unknown_field(origin)).is_err()
    );

    let rejected = serde_json::to_value(HistoryNavigationResult::Rejected {
        snapshot,
        rejection: HistoryNavigationRejectionProjection {
            code: HistoryNavigationRejectionCode::StaleRevision,
            detail: "stale".to_owned(),
            refresh_required: true,
        },
    })
    .unwrap();
    assert!(serde_json::from_value::<HistoryNavigationResult>(rejected.clone()).is_ok());
    assert!(
        serde_json::from_value::<HistoryNavigationResult>(with_unknown_field(rejected)).is_err()
    );
}
