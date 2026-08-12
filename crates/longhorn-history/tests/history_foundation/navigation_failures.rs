use std::{convert::Infallible, error::Error, fmt};

use longhorn_core::{HistoryPlanId, HistoryRevision};
use longhorn_history::{
    HistoryCoalesce, HistoryLimits, HistoryNavigationExecutionError, HistoryNavigationLimits,
    HistoryNavigationPlan, HistoryNavigationPlanningError, HistoryNavigationRejection,
    HistoryNavigationRequest, HistoryNavigationTarget, HistoryNavigationTransaction,
    HistoryNavigationTransactionFailure, HistoryPolicy, LinearHistory,
};

use crate::support::*;

#[derive(Clone, Debug, Eq, PartialEq)]
struct DocumentMutation {
    before: i32,
    after: i32,
}

struct DocumentPolicy;

impl HistoryPolicy<DocumentMutation> for DocumentPolicy {
    type Error = Infallible;

    fn inverse(&self, payload: &DocumentMutation) -> Result<DocumentMutation, Self::Error> {
        Ok(DocumentMutation {
            before: payload.after,
            after: payload.before,
        })
    }

    fn is_noop(&self, payload: &DocumentMutation) -> bool {
        payload.before == payload.after
    }

    fn encoded_weight(&self, _: &DocumentMutation) -> Result<u64, Self::Error> {
        Ok(1)
    }

    fn coalesce(
        &self,
        _: &DocumentMutation,
        _: &DocumentMutation,
        _: longhorn_history::HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<DocumentMutation>, Self::Error> {
        Ok(HistoryCoalesce::KeepSeparate)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DocumentTransactionError {
    InjectedApply(usize),
    UnexpectedValue,
    InjectedRollback,
}

impl fmt::Display for DocumentTransactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for DocumentTransactionError {}

struct DocumentTransaction {
    value: i32,
    fail_at_step: Option<usize>,
    rollback_fails: bool,
    apply_calls: usize,
}

impl DocumentTransaction {
    fn successful(value: i32) -> Self {
        Self {
            value,
            fail_at_step: None,
            rollback_fails: false,
            apply_calls: 0,
        }
    }
}

impl HistoryNavigationTransaction<DocumentMutation> for DocumentTransaction {
    type Error = DocumentTransactionError;

    fn apply(
        &mut self,
        plan: &HistoryNavigationPlan<DocumentMutation>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        self.apply_calls += 1;
        let source = self.value;
        for (index, step) in plan.steps().iter().enumerate() {
            if self.fail_at_step == Some(index) {
                let error = DocumentTransactionError::InjectedApply(index);
                if self.rollback_fails {
                    return Err(HistoryNavigationTransactionFailure::RollbackFailed {
                        error,
                        rollback_error: DocumentTransactionError::InjectedRollback,
                    });
                }
                self.value = source;
                return Err(HistoryNavigationTransactionFailure::RolledBack { error });
            }
            if self.value != step.payload().before {
                self.value = source;
                return Err(HistoryNavigationTransactionFailure::RolledBack {
                    error: DocumentTransactionError::UnexpectedValue,
                });
            }
            self.value = step.payload().after;
        }
        Ok(())
    }
}

fn plan_id(value: &str) -> HistoryPlanId {
    HistoryPlanId::new(value).expect("fixture plan id")
}

fn request(
    value: &str,
    revision: u64,
    target: HistoryNavigationTarget,
) -> HistoryNavigationRequest {
    HistoryNavigationRequest::new(plan_id(value), HistoryRevision::new(revision), target)
}

fn document_history(navigation_limits: HistoryNavigationLimits) -> LinearHistory<DocumentMutation> {
    let mut history = LinearHistory::with_navigation_limits(
        history_id("history:document-navigation"),
        HistoryLimits::default(),
        navigation_limits,
    );
    for (index, (before, after)) in [(0, 1), (1, 2), (2, 3)].into_iter().enumerate() {
        history
            .record_applied(
                record(
                    index as u64,
                    &format!("entry:{}", index + 1),
                    metadata(&format!("Set value to {after}"), "document:set"),
                    DocumentMutation { before, after },
                ),
                &DocumentPolicy,
            )
            .unwrap();
    }
    history
}

#[test]
fn multi_entry_apply_failure_and_rollback_failure_leave_history_exact() {
    let mut history = document_history(HistoryNavigationLimits::DEFAULT);
    let before_history = history.clone();
    let plan = history
        .plan_navigation(
            request(
                "plan:rollback",
                3,
                HistoryNavigationTarget::Checkout {
                    entry_id: entry_id("entry:1"),
                },
            ),
            &DocumentPolicy,
        )
        .unwrap();
    let mut transaction = DocumentTransaction {
        value: 3,
        fail_at_step: Some(1),
        rollback_fails: false,
        apply_calls: 0,
    };

    assert_eq!(
        history.execute_navigation(plan, &mut transaction),
        Err(HistoryNavigationExecutionError::RolledBack {
            plan_id: plan_id("plan:rollback"),
            error: DocumentTransactionError::InjectedApply(1),
        })
    );
    assert_eq!(history, before_history);
    assert_eq!(transaction.value, 3);

    let mut history = document_history(HistoryNavigationLimits::DEFAULT);
    let before_history = history.clone();
    let plan = history
        .plan_navigation(
            request(
                "plan:rollback-fails",
                3,
                HistoryNavigationTarget::Checkout {
                    entry_id: entry_id("entry:1"),
                },
            ),
            &DocumentPolicy,
        )
        .unwrap();
    let mut transaction = DocumentTransaction {
        value: 3,
        fail_at_step: Some(1),
        rollback_fails: true,
        apply_calls: 0,
    };

    assert_eq!(
        history.execute_navigation(plan, &mut transaction),
        Err(HistoryNavigationExecutionError::RollbackFailed {
            plan_id: plan_id("plan:rollback-fails"),
            error: DocumentTransactionError::InjectedApply(1),
            rollback_error: DocumentTransactionError::InjectedRollback,
        })
    );
    assert_eq!(history, before_history);
    assert_eq!(transaction.value, 2);
}

#[test]
fn stale_and_duplicate_plans_reject_before_product_apply() {
    let mut history = document_history(HistoryNavigationLimits::DEFAULT);
    let stale = history
        .plan_navigation(
            request("plan:stale", 3, HistoryNavigationTarget::Undo),
            &DocumentPolicy,
        )
        .unwrap();
    history
        .record_applied(
            record(
                3,
                "entry:4",
                metadata("Set value to 4", "document:set"),
                DocumentMutation {
                    before: 3,
                    after: 4,
                },
            ),
            &DocumentPolicy,
        )
        .unwrap();
    let before = history.clone();
    let mut transaction = DocumentTransaction::successful(4);
    assert!(matches!(
        history.execute_navigation(stale, &mut transaction),
        Err(HistoryNavigationExecutionError::Rejected {
            rejection: HistoryNavigationRejection::StaleRevision { .. },
            ..
        })
    ));
    assert_eq!(transaction.apply_calls, 0);
    assert_eq!(history, before);

    let plan = history
        .plan_navigation(
            request("plan:duplicate", 4, HistoryNavigationTarget::Undo),
            &DocumentPolicy,
        )
        .unwrap();
    let duplicate = plan.clone();
    history.execute_navigation(plan, &mut transaction).unwrap();
    let after_commit = history.clone();
    assert!(matches!(
        history.execute_navigation(duplicate, &mut transaction),
        Err(HistoryNavigationExecutionError::Rejected {
            rejection: HistoryNavigationRejection::DuplicatePlan,
            ..
        })
    ));
    assert_eq!(transaction.apply_calls, 1);
    assert_eq!(history, after_commit);
    assert_eq!(
        history.plan_navigation(
            request("plan:duplicate", 5, HistoryNavigationTarget::Redo),
            &DocumentPolicy,
        ),
        Err(HistoryNavigationPlanningError::DuplicatePlanId(plan_id(
            "plan:duplicate"
        )))
    );
}

#[test]
fn checkout_is_bounded_and_stable_ids_are_the_only_authority() {
    let limits = HistoryNavigationLimits::new(1, 4).unwrap();
    let history = document_history(limits);
    let before = history.clone();
    assert_eq!(
        history.plan_navigation(
            request(
                "plan:too-deep",
                3,
                HistoryNavigationTarget::Checkout {
                    entry_id: entry_id("entry:1"),
                },
            ),
            &DocumentPolicy,
        ),
        Err(HistoryNavigationPlanningError::TooManySteps {
            maximum: 1,
            actual: 2,
        })
    );
    assert_eq!(
        history.plan_navigation(
            request(
                "plan:missing",
                3,
                HistoryNavigationTarget::Checkout {
                    entry_id: entry_id("entry:missing"),
                },
            ),
            &DocumentPolicy,
        ),
        Err(HistoryNavigationPlanningError::UnknownEntry(entry_id(
            "entry:missing"
        )))
    );
    assert_eq!(history, before);
}

#[test]
fn explicit_checkout_of_current_entry_is_a_zero_step_commit() {
    let mut history = document_history(HistoryNavigationLimits::DEFAULT);
    let plan = history
        .plan_navigation(
            request(
                "plan:current",
                3,
                HistoryNavigationTarget::Checkout {
                    entry_id: entry_id("entry:3"),
                },
            ),
            &DocumentPolicy,
        )
        .unwrap();
    assert!(plan.steps().is_empty());
    let mut transaction = DocumentTransaction::successful(3);
    let receipt = history.execute_navigation(plan, &mut transaction).unwrap();

    assert_eq!(transaction.value, 3);
    assert_eq!(transaction.apply_calls, 1);
    assert_eq!(receipt.previous_revision().get(), 3);
    assert_eq!(receipt.committed_revision().get(), 4);
    assert_eq!(receipt.moved_entry_ids(), &[]);
    assert_eq!(receipt.authoritative_position(), receipt.source_position());
}

#[test]
fn empty_and_revision_exhausted_histories_fail_during_planning() {
    let empty = LinearHistory::<DocumentMutation>::new(
        history_id("history:empty"),
        HistoryLimits::default(),
    );
    assert_eq!(
        empty.plan_navigation(
            request("plan:no-undo", 0, HistoryNavigationTarget::Undo),
            &DocumentPolicy,
        ),
        Err(HistoryNavigationPlanningError::NothingToUndo)
    );
    assert_eq!(
        empty.plan_navigation(
            request("plan:no-redo", 0, HistoryNavigationTarget::Redo),
            &DocumentPolicy,
        ),
        Err(HistoryNavigationPlanningError::NothingToRedo)
    );

    let state = longhorn_history::LinearHistoryState::new(
        history_id("history:revision-max"),
        HistoryRevision::new(u64::MAX),
        longhorn_history::HistoryEntrySequence::new(2).unwrap(),
        vec![entry(
            "entry:1",
            "Set value",
            "document:set",
            1,
            u64::MAX,
            DocumentMutation {
                before: 0,
                after: 1,
            },
        )],
        Vec::new(),
    );
    let history = LinearHistory::from_state(HistoryLimits::default(), state).unwrap();
    assert_eq!(
        history.plan_navigation(
            request(
                "plan:revision-overflow",
                u64::MAX,
                HistoryNavigationTarget::Undo,
            ),
            &DocumentPolicy,
        ),
        Err(HistoryNavigationPlanningError::RevisionOverflow)
    );
}

// --- Card 191: the origin is a position the operator can name ---------------

/// Three entries, and the state the document was opened in. Reaching it meant
/// one undo per entry, which for a real history is not a control an operator
/// can use.
#[test]
fn checkout_root_unwinds_every_applied_entry_in_one_plan() {
    let mut history = document_history(HistoryNavigationLimits::DEFAULT);
    let plan = history
        .plan_navigation(
            request("plan:origin", 3, HistoryNavigationTarget::CheckoutRoot),
            &DocumentPolicy,
        )
        .expect("the origin is reachable");
    assert_eq!(plan.steps().len(), 3, "one inverse per applied entry");

    let mut transaction = DocumentTransaction {
        value: 3,
        fail_at_step: None,
        rollback_fails: false,
        apply_calls: 0,
    };
    history
        .execute_navigation(plan, &mut transaction)
        .expect("origin commit");
    assert_eq!(transaction.value, 0, "back to the state before entry:1");

    let summary = history.project_summary().expect("summary");
    assert_eq!(summary.current_entry_id(), None);
    assert_eq!(summary.undo_depth(), 0);
    assert_eq!(summary.redo_depth(), 3, "everything is redoable from here");
}

#[test]
fn checkout_root_from_the_origin_is_refused() {
    let mut history = document_history(HistoryNavigationLimits::DEFAULT);
    let plan = history
        .plan_navigation(
            request("plan:first", 3, HistoryNavigationTarget::CheckoutRoot),
            &DocumentPolicy,
        )
        .expect("first descent");
    history
        .execute_navigation(
            plan,
            &mut DocumentTransaction {
                value: 3,
                fail_at_step: None,
                rollback_fails: false,
                apply_calls: 0,
            },
        )
        .expect("origin commit");

    let error = history
        .plan_navigation(
            request("plan:again", 4, HistoryNavigationTarget::CheckoutRoot),
            &DocumentPolicy,
        )
        .expect_err("already at the origin");
    assert!(matches!(
        error,
        HistoryNavigationPlanningError::NothingToUndo
    ));
}
