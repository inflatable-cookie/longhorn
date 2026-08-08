use longhorn_core::{HistoryEntryId, HistoryId, HistoryPlanId, HistoryRevision};

use crate::{HistoryLimits, HistoryNavigationTarget, LinearHistory};

use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Payload(u8);

#[test]
fn corrupted_private_plan_rejects_before_transaction() {
    let history = LinearHistory::<Payload>::new(
        HistoryId::new("history:test").unwrap(),
        HistoryLimits::default(),
    );
    let mut plan = HistoryNavigationPlan {
        history_id: history.history_id().clone(),
        plan_id: HistoryPlanId::new("plan:test").unwrap(),
        source_revision: HistoryRevision::INITIAL,
        target: HistoryNavigationTarget::Checkout {
            entry_id: HistoryEntryId::new("entry:missing").unwrap(),
        },
        direction: HistoryNavigationDirection::Stationary,
        source_position: history.position_at(0),
        target_position: history.position_at(0),
        steps: Vec::new(),
    };
    plan.target_position.applied_depth = 1;

    assert_eq!(
        history.validate_plan(&plan),
        Err(HistoryNavigationRejection::InvalidPlan)
    );
}
