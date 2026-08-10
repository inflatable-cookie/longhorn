use std::{collections::BTreeMap, error::Error, fmt};

use longhorn_core::{HistoryPlanId, HistoryRevision};
use longhorn_history::{
    HistoryLimits, HistoryNavigationDirection, HistoryNavigationPlan,
    HistoryNavigationPlanningError, HistoryNavigationRequest, HistoryNavigationStep,
    HistoryNavigationTarget, HistoryNavigationTransaction, HistoryNavigationTransactionFailure,
    LinearHistory,
};

use crate::{
    pulse_shaped::{PulseFixtureMutation, PulseFixturePolicy, rename},
    support::*,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PulseTransactionError {
    MissingTrack(u32),
    UnexpectedName(u32),
    InjectedApply(usize),
    InjectedRollback,
}

impl fmt::Display for PulseTransactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for PulseTransactionError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FailureMode {
    None,
    RollBackAt(usize),
    FailRollbackAt(usize),
}

struct PulseTransaction {
    tracks: BTreeMap<u32, String>,
    failure_mode: FailureMode,
    apply_calls: usize,
}

impl PulseTransaction {
    fn final_model() -> Self {
        Self {
            tracks: BTreeMap::from([(1, "Beats".to_owned()), (2, "Low".to_owned())]),
            failure_mode: FailureMode::None,
            apply_calls: 0,
        }
    }

    fn apply_payload(
        &mut self,
        payload: &PulseFixtureMutation,
        operation: &mut usize,
    ) -> Result<(), PulseTransactionError> {
        match payload {
            PulseFixtureMutation::RenameTrack {
                track_id,
                before,
                after,
            } => {
                self.maybe_fail(*operation)?;
                *operation += 1;
                let current = self
                    .tracks
                    .get_mut(track_id)
                    .ok_or(PulseTransactionError::MissingTrack(*track_id))?;
                if current != before {
                    return Err(PulseTransactionError::UnexpectedName(*track_id));
                }
                *current = after.clone();
                Ok(())
            }
            PulseFixtureMutation::DeleteTrack { track_id, .. } => {
                self.maybe_fail(*operation)?;
                *operation += 1;
                self.tracks
                    .remove(track_id)
                    .ok_or(PulseTransactionError::MissingTrack(*track_id))?;
                Ok(())
            }
            PulseFixtureMutation::RestoreTrack { track_id, snapshot } => {
                self.maybe_fail(*operation)?;
                *operation += 1;
                self.tracks.insert(*track_id, snapshot.clone());
                Ok(())
            }
            PulseFixtureMutation::Compound { mutations } => {
                for mutation in mutations {
                    self.apply_payload(mutation, operation)?;
                }
                Ok(())
            }
            PulseFixtureMutation::Unsupported => {
                Err(PulseTransactionError::InjectedApply(*operation))
            }
        }
    }

    fn maybe_fail(&self, operation: usize) -> Result<(), PulseTransactionError> {
        match self.failure_mode {
            FailureMode::RollBackAt(target) | FailureMode::FailRollbackAt(target)
                if target == operation =>
            {
                Err(PulseTransactionError::InjectedApply(operation))
            }
            _ => Ok(()),
        }
    }
}

impl HistoryNavigationTransaction<PulseFixtureMutation> for PulseTransaction {
    type Error = PulseTransactionError;

    fn apply(
        &mut self,
        plan: &HistoryNavigationPlan<PulseFixtureMutation>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        self.apply_calls += 1;
        let source = self.tracks.clone();
        let mut operation = 0;
        for step in plan.steps() {
            if let Err(error) = self.apply_payload(step.payload(), &mut operation) {
                return match self.failure_mode {
                    FailureMode::FailRollbackAt(_) => {
                        Err(HistoryNavigationTransactionFailure::RollbackFailed {
                            error,
                            rollback_error: PulseTransactionError::InjectedRollback,
                        })
                    }
                    FailureMode::None | FailureMode::RollBackAt(_) => {
                        self.tracks = source;
                        Err(HistoryNavigationTransactionFailure::RolledBack { error })
                    }
                };
            }
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

fn pulse_history() -> LinearHistory<PulseFixtureMutation> {
    let mut history = LinearHistory::new(history_id("history:pulse"), HistoryLimits::default());
    history
        .record_applied(
            record(
                0,
                "entry:0001",
                metadata("Rename drums", "track:rename"),
                rename(1, "Drums", "Kit"),
            ),
            &PulseFixturePolicy,
        )
        .unwrap();
    history
        .record_applied(
            record(
                1,
                "entry:0002",
                metadata("Rename bass", "track:rename"),
                rename(2, "Bass", "Sub"),
            ),
            &PulseFixturePolicy,
        )
        .unwrap();
    history
        .record_applied(
            record(
                2,
                "entry:0003",
                metadata("Rename mix tracks", "track:compound"),
                PulseFixtureMutation::Compound {
                    mutations: vec![rename(1, "Kit", "Beats"), rename(2, "Sub", "Low")],
                },
            ),
            &PulseFixturePolicy,
        )
        .unwrap();
    history
}

#[test]
fn pulse_shaped_undo_redo_and_checkout_keep_exact_order() {
    let policy = PulseFixturePolicy;
    let mut history = pulse_history();
    let initial_history = history.clone();
    let mut transaction = PulseTransaction::final_model();

    let undo = history
        .plan_navigation(
            request("plan:undo", 3, HistoryNavigationTarget::Undo),
            &policy,
        )
        .unwrap();
    assert_eq!(history, initial_history);
    assert_eq!(undo.direction(), HistoryNavigationDirection::Undo);
    assert_eq!(undo.steps().len(), 1);
    assert_eq!(undo.steps()[0].entry_id(), &entry_id("entry:0003"));
    let HistoryNavigationStep::Undo { payload, .. } = &undo.steps()[0] else {
        panic!("expected undo step");
    };
    assert_eq!(
        payload,
        &PulseFixtureMutation::Compound {
            mutations: vec![rename(2, "Low", "Sub"), rename(1, "Beats", "Kit"),],
        }
    );

    let undo_receipt = history.execute_navigation(undo, &mut transaction).unwrap();
    assert_eq!(transaction.tracks[&1], "Kit");
    assert_eq!(transaction.tracks[&2], "Sub");
    assert_eq!(history.applied().len(), 2);
    assert_eq!(history.future().len(), 1);
    assert_eq!(undo_receipt.previous_revision().get(), 3);
    assert_eq!(undo_receipt.committed_revision().get(), 4);
    assert_eq!(
        undo_receipt
            .authoritative_position()
            .next_undo_label()
            .unwrap()
            .as_str(),
        "Rename bass"
    );
    assert_eq!(
        undo_receipt
            .authoritative_position()
            .next_redo_label()
            .unwrap()
            .as_str(),
        "Rename mix tracks"
    );

    let redo = history
        .plan_navigation(
            request("plan:redo", 4, HistoryNavigationTarget::Redo),
            &policy,
        )
        .unwrap();
    assert_eq!(
        redo.steps()[0].direction(),
        HistoryNavigationDirection::Redo
    );
    history.execute_navigation(redo, &mut transaction).unwrap();
    assert_eq!(transaction.tracks[&1], "Beats");
    assert_eq!(transaction.tracks[&2], "Low");

    let checkout_back = history
        .plan_navigation(
            request(
                "plan:checkout-back",
                5,
                HistoryNavigationTarget::Checkout {
                    entry_id: entry_id("entry:0001"),
                },
            ),
            &policy,
        )
        .unwrap();
    assert_eq!(
        checkout_back
            .steps()
            .iter()
            .map(|step| step.entry_id().as_str())
            .collect::<Vec<_>>(),
        vec!["entry:0003", "entry:0002"]
    );
    history
        .execute_navigation(checkout_back, &mut transaction)
        .unwrap();
    assert_eq!(transaction.tracks[&1], "Kit");
    assert_eq!(transaction.tracks[&2], "Bass");
    assert_eq!(
        history.current().unwrap().entry_id(),
        &entry_id("entry:0001")
    );

    let checkout_forward = history
        .plan_navigation(
            request(
                "plan:checkout-forward",
                6,
                HistoryNavigationTarget::Checkout {
                    entry_id: entry_id("entry:0003"),
                },
            ),
            &policy,
        )
        .unwrap();
    assert_eq!(
        checkout_forward
            .steps()
            .iter()
            .map(|step| step.entry_id().as_str())
            .collect::<Vec<_>>(),
        vec!["entry:0002", "entry:0003"]
    );
    let receipt = history
        .execute_navigation(checkout_forward, &mut transaction)
        .unwrap();
    assert_eq!(transaction.tracks[&1], "Beats");
    assert_eq!(transaction.tracks[&2], "Low");
    assert_eq!(receipt.authoritative_position().applied_depth(), 3);
    assert_eq!(receipt.authoritative_position().future_depth(), 0);
}

#[test]
fn pulse_shaped_compound_failure_never_partially_commits_history() {
    let policy = PulseFixturePolicy;
    let mut history = pulse_history();
    let before_history = history.clone();
    let mut transaction = PulseTransaction::final_model();
    transaction.failure_mode = FailureMode::RollBackAt(1);
    let before_model = transaction.tracks.clone();
    let plan = history
        .plan_navigation(
            request("plan:compound-rollback", 3, HistoryNavigationTarget::Undo),
            &policy,
        )
        .unwrap();

    assert!(matches!(
        history.execute_navigation(plan, &mut transaction),
        Err(longhorn_history::HistoryNavigationExecutionError::RolledBack { .. })
    ));
    assert_eq!(history, before_history);
    assert_eq!(transaction.tracks, before_model);

    let mut history = pulse_history();
    let before_history = history.clone();
    let mut transaction = PulseTransaction::final_model();
    transaction.failure_mode = FailureMode::FailRollbackAt(1);
    let plan = history
        .plan_navigation(
            request(
                "plan:compound-rollback-fails",
                3,
                HistoryNavigationTarget::Undo,
            ),
            &policy,
        )
        .unwrap();

    assert!(matches!(
        history.execute_navigation(plan, &mut transaction),
        Err(longhorn_history::HistoryNavigationExecutionError::RollbackFailed { .. })
    ));
    assert_eq!(history, before_history);
    assert_eq!(transaction.tracks[&1], "Beats");
    assert_eq!(transaction.tracks[&2], "Sub");
}

#[test]
fn pulse_shaped_owned_inverse_rejection_never_produces_a_plan() {
    let mut history = LinearHistory::new(history_id("history:pulse"), HistoryLimits::default());
    history
        .record_applied(
            record(
                0,
                "entry:unsupported",
                metadata("Unsupported fixture", "fixture:unsupported"),
                PulseFixtureMutation::Unsupported,
            ),
            &PulseFixturePolicy,
        )
        .unwrap();
    let before = history.clone();

    assert!(matches!(
        history.plan_navigation(
            request("plan:unsupported", 1, HistoryNavigationTarget::Undo),
            &PulseFixturePolicy,
        ),
        Err(HistoryNavigationPlanningError::Policy {
            entry_id: rejected,
            ..
        }) if rejected == entry_id("entry:unsupported")
    ));
    assert_eq!(history, before);
}
