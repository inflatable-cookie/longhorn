//! Atomic navigation, protected retention, checkpoint, and depth evidence.

use longhorn_core::{HistoryEntryId, HistoryId, HistoryKindId, HistoryPlanId, HistoryRevision};
use longhorn_history::{
    HistoryCoalesce, HistoryCoalesceContext, HistoryEntryMetadata, HistoryLabel,
    HistoryNavigationStep, HistoryNavigationTransactionFailure, HistoryPolicy,
};
use longhorn_history_tree::{
    ForkBranchId, ForkBranchMetadata, ForkBranchSeed, ForkCheckpoint, ForkCheckpointError,
    ForkCheckpointId, ForkHistory, ForkHistoryState, ForkHistoryStateError, ForkNavigationError,
    ForkNavigationPlan, ForkNavigationTarget, ForkNavigationTransaction, ForkPruningOutcome,
    ForkRecord, ForkRetentionError, ForkRetentionLimits,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Delta(i64);

struct DeltaPolicy;

impl HistoryPolicy<Delta> for DeltaPolicy {
    type Error = &'static str;

    fn inverse(&self, payload: &Delta) -> Result<Delta, Self::Error> {
        Ok(Delta(-payload.0))
    }

    fn is_noop(&self, payload: &Delta) -> bool {
        payload.0 == 0
    }

    fn encoded_weight(&self, _payload: &Delta) -> Result<u64, Self::Error> {
        Ok(8)
    }

    fn coalesce(
        &self,
        _previous: &Delta,
        _incoming: &Delta,
        _context: HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<Delta>, Self::Error> {
        Ok(HistoryCoalesce::KeepSeparate)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TransactionMode {
    Commit,
    RollBack,
    RollbackFails,
}

struct ModelTransaction<'model> {
    model: &'model mut i64,
    mode: TransactionMode,
    calls: usize,
}

impl ForkNavigationTransaction<Delta> for ModelTransaction<'_> {
    type Error = &'static str;

    fn apply(
        &mut self,
        plan: &ForkNavigationPlan<Delta>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        self.calls += 1;
        let source = *self.model;
        for step in plan.steps() {
            let delta = match step {
                HistoryNavigationStep::Undo { payload, .. }
                | HistoryNavigationStep::Redo { payload, .. } => payload.0,
            };
            *self.model += delta;
            if self.mode != TransactionMode::Commit {
                break;
            }
        }
        match self.mode {
            TransactionMode::Commit => Ok(()),
            TransactionMode::RollBack => {
                *self.model = source;
                Err(HistoryNavigationTransactionFailure::RolledBack {
                    error: "apply failed",
                })
            }
            TransactionMode::RollbackFails => {
                Err(HistoryNavigationTransactionFailure::RollbackFailed {
                    error: "apply failed",
                    rollback_error: "rollback failed",
                })
            }
        }
    }
}

fn branch_id(value: &str) -> ForkBranchId {
    ForkBranchId::new(value).expect("fixture branch id")
}

fn checkpoint_id(value: &str) -> ForkCheckpointId {
    ForkCheckpointId::new(value).expect("fixture checkpoint id")
}

fn entry_id(value: &str) -> HistoryEntryId {
    HistoryEntryId::new(value).expect("fixture entry id")
}

fn plan_id(value: &str) -> HistoryPlanId {
    HistoryPlanId::new(value).expect("fixture plan id")
}

fn metadata(label: &str) -> HistoryEntryMetadata {
    HistoryEntryMetadata::new(
        HistoryLabel::new(label).expect("fixture label"),
        Some(HistoryKindId::new("fixture:delta").expect("fixture kind")),
        None,
    )
}

fn branch_metadata(name: Option<&str>, pinned: bool) -> ForkBranchMetadata {
    ForkBranchMetadata::new(name.map(str::to_owned), None, pinned).expect("fixture branch metadata")
}

fn history() -> ForkHistory<Delta> {
    ForkHistory::new(
        HistoryId::new("history:navigation").expect("fixture history id"),
        branch_id("branch:main"),
        branch_metadata(Some("Main"), true),
    )
}

fn record(
    history: &mut ForkHistory<Delta>,
    model: &mut i64,
    id: &str,
    delta: i64,
    divergent_branch: Option<ForkBranchSeed>,
) {
    *model += delta;
    history
        .record_applied(ForkRecord::new(
            history.revision(),
            entry_id(id),
            metadata(id),
            8,
            Delta(delta),
            divergent_branch,
        ))
        .expect("fixture record");
}

fn navigate(
    history: &mut ForkHistory<Delta>,
    model: &mut i64,
    id: &str,
    target: ForkNavigationTarget,
) {
    let plan = history
        .plan_navigation(plan_id(id), history.revision(), target, &DeltaPolicy)
        .expect("fixture plan");
    history
        .execute_navigation(
            plan,
            &mut ModelTransaction {
                model,
                mode: TransactionMode::Commit,
                calls: 0,
            },
        )
        .expect("fixture navigation");
}

fn forked_history(alternate_metadata: ForkBranchMetadata) -> (ForkHistory<Delta>, i64) {
    let mut history = history();
    let mut model = 0;
    record(&mut history, &mut model, "entry:a", 1, None);
    record(&mut history, &mut model, "entry:b", 2, None);
    record(&mut history, &mut model, "entry:c", 3, None);
    navigate(
        &mut history,
        &mut model,
        "plan:undo-c",
        ForkNavigationTarget::Undo,
    );
    record(
        &mut history,
        &mut model,
        "entry:d",
        4,
        Some(ForkBranchSeed::new(
            branch_id("branch:alternate"),
            alternate_metadata,
        )),
    );
    (history, model)
}

#[test]
fn mixed_lca_checkout_commits_one_complete_route_and_preferred_redo() {
    let (mut history, mut model) = forked_history(branch_metadata(Some("Alternate"), false));
    assert_eq!(model, 7);
    let plan = history
        .plan_navigation(
            plan_id("plan:checkout-main"),
            history.revision(),
            ForkNavigationTarget::Checkout {
                branch_id: branch_id("branch:main"),
                entry_id: entry_id("entry:c"),
            },
            &DeltaPolicy,
        )
        .expect("mixed checkout plan");
    assert_eq!(plan.lowest_common_ancestor(), Some(&entry_id("entry:b")));
    assert!(matches!(
        &plan.steps()[0],
        HistoryNavigationStep::Undo { entry_id: id, payload }
            if id == &entry_id("entry:d") && payload == &Delta(-4)
    ));
    assert!(matches!(
        &plan.steps()[1],
        HistoryNavigationStep::Redo { entry_id: id, payload }
            if id == &entry_id("entry:c") && payload == &Delta(3)
    ));
    let source_revision = history.revision();
    let mut transaction = ModelTransaction {
        model: &mut model,
        mode: TransactionMode::Commit,
        calls: 0,
    };
    let receipt = history
        .execute_navigation(plan, &mut transaction)
        .expect("mixed checkout commit");
    assert_eq!(transaction.calls, 1);
    assert_eq!(model, 6);
    assert_eq!(receipt.previous_revision(), source_revision);
    assert_eq!(receipt.target_branch_id(), &branch_id("branch:main"));
    assert_eq!(receipt.target_node_id(), Some(&entry_id("entry:c")));
    assert_eq!(
        receipt.moved_entry_ids(),
        &[entry_id("entry:d"), entry_id("entry:c")]
    );

    navigate(
        &mut history,
        &mut model,
        "plan:back-to-b",
        ForkNavigationTarget::Undo,
    );
    let redo = history
        .plan_navigation(
            plan_id("plan:preferred-main"),
            history.revision(),
            ForkNavigationTarget::Redo,
            &DeltaPolicy,
        )
        .expect("preferred redo");
    assert_eq!(redo.target_branch_id(), &branch_id("branch:main"));
    assert_eq!(redo.target_node_id(), Some(&entry_id("entry:c")));
}

#[test]
fn apply_rollback_stale_and_rollback_failure_preserve_graph_authority() {
    for mode in [TransactionMode::RollBack, TransactionMode::RollbackFails] {
        let (mut history, mut model) = forked_history(branch_metadata(Some("Alternate"), false));
        let before_history = history.clone();
        let before_model = model;
        let plan = history
            .plan_navigation(
                plan_id("plan:failure"),
                history.revision(),
                ForkNavigationTarget::Checkout {
                    branch_id: branch_id("branch:main"),
                    entry_id: entry_id("entry:c"),
                },
                &DeltaPolicy,
            )
            .expect("failure plan");
        let error = history
            .execute_navigation(
                plan,
                &mut ModelTransaction {
                    model: &mut model,
                    mode,
                    calls: 0,
                },
            )
            .expect_err("transaction failure");
        assert_eq!(history, before_history);
        match mode {
            TransactionMode::RollBack => {
                assert_eq!(model, before_model);
                assert_eq!(
                    error,
                    ForkNavigationError::RolledBack {
                        error: "apply failed"
                    }
                );
            }
            TransactionMode::RollbackFails => {
                assert_ne!(model, before_model);
                assert_eq!(
                    error,
                    ForkNavigationError::RollbackFailed {
                        error: "apply failed",
                        rollback_error: "rollback failed"
                    }
                );
            }
            TransactionMode::Commit => unreachable!(),
        }
    }

    let (mut history, mut model) = forked_history(branch_metadata(Some("Alternate"), false));
    let plan = history
        .plan_navigation(
            plan_id("plan:stale"),
            history.revision(),
            ForkNavigationTarget::Checkout {
                branch_id: branch_id("branch:main"),
                entry_id: entry_id("entry:c"),
            },
            &DeltaPolicy,
        )
        .expect("stale plan");
    history
        .set_branch_metadata(
            history.revision(),
            &branch_id("branch:alternate"),
            branch_metadata(Some("Changed"), false),
        )
        .expect("intervening commit");
    let before = history.clone();
    let mut transaction = ModelTransaction {
        model: &mut model,
        mode: TransactionMode::Commit,
        calls: 0,
    };
    assert!(matches!(
        history.execute_navigation(plan, &mut transaction),
        Err(ForkNavigationError::StaleRevision { .. })
    ));
    assert_eq!(transaction.calls, 0);
    assert_eq!(history, before);
}

#[test]
fn deterministic_pruning_removes_only_anonymous_unpinned_future() {
    let (mut history, mut model) = forked_history(branch_metadata(None, false));
    record(&mut history, &mut model, "entry:e", 5, None);
    history
        .register_checkpoint(
            history.revision(),
            checkpoint_id("checkpoint:alternate"),
            Some(entry_id("entry:e")),
            "consumer://alternate".to_owned(),
        )
        .expect("alternate checkpoint");
    navigate(
        &mut history,
        &mut model,
        "plan:select-main",
        ForkNavigationTarget::Checkout {
            branch_id: branch_id("branch:main"),
            entry_id: entry_id("entry:c"),
        },
    );
    let source_revision = history.revision();
    let outcome = history
        .prune_to(
            source_revision,
            ForkRetentionLimits::new(3, 24).expect("fixture limits"),
        )
        .expect("bounded prune");
    let ForkPruningOutcome::Pruned(receipt) = outcome else {
        panic!("expected pruning")
    };
    assert_eq!(
        receipt
            .pruned_nodes()
            .iter()
            .map(|node| node.entry_id().clone())
            .collect::<Vec<_>>(),
        vec![entry_id("entry:e"), entry_id("entry:d")]
    );
    assert_eq!(receipt.removed_branches(), &[branch_id("branch:alternate")]);
    assert_eq!(
        receipt.removed_checkpoints(),
        &[checkpoint_id("checkpoint:alternate")]
    );
    assert_eq!(receipt.retained_entry_count(), 3);
    assert_eq!(receipt.retained_encoded_weight(), 24);
    assert_eq!(history.current_node_id(), Some(&entry_id("entry:c")));
    assert!(history.node(&entry_id("entry:d")).is_none());

    navigate(
        &mut history,
        &mut model,
        "plan:undo-after-prune",
        ForkNavigationTarget::Undo,
    );
    let redo = history
        .plan_navigation(
            plan_id("plan:redo-after-prune"),
            history.revision(),
            ForkNavigationTarget::Redo,
            &DeltaPolicy,
        )
        .expect("surviving preferred redo");
    assert_eq!(redo.target_branch_id(), &branch_id("branch:main"));
    assert_eq!(redo.target_node_id(), Some(&entry_id("entry:c")));
}

#[test]
fn named_pinned_and_current_lineages_reject_impossible_budgets_without_mutation() {
    for metadata in [
        branch_metadata(Some("Named"), false),
        branch_metadata(None, true),
    ] {
        let (mut history, mut model) = forked_history(metadata);
        navigate(
            &mut history,
            &mut model,
            "plan:select-main",
            ForkNavigationTarget::Checkout {
                branch_id: branch_id("branch:main"),
                entry_id: entry_id("entry:c"),
            },
        );
        let before = history.clone();
        let error = history
            .prune_to(
                history.revision(),
                ForkRetentionLimits::new(3, 24).expect("fixture limits"),
            )
            .expect_err("protected budget");
        assert!(matches!(error, ForkRetentionError::ProtectedBudget { .. }));
        assert_eq!(history, before);
    }

    let (mut history, _model) = forked_history(branch_metadata(None, false));
    let before = history.clone();
    assert!(matches!(
        history.prune_to(
            history.revision(),
            ForkRetentionLimits::new(3, 24).expect("fixture limits")
        ),
        Err(ForkRetentionError::ProtectedBudget { .. })
    ));
    assert_eq!(history, before);
}

#[test]
fn checkpoints_are_opaque_bounded_and_choose_nearest_ancestor() {
    let mut history = history();
    let mut model = 0;
    record(&mut history, &mut model, "entry:a", 1, None);
    record(&mut history, &mut model, "entry:b", 2, None);
    record(&mut history, &mut model, "entry:c", 3, None);
    history
        .register_checkpoint(
            history.revision(),
            checkpoint_id("checkpoint:root"),
            None,
            "snapshot://root".to_owned(),
        )
        .expect("root checkpoint");
    history
        .register_checkpoint(
            history.revision(),
            checkpoint_id("checkpoint:b"),
            Some(entry_id("entry:b")),
            "snapshot://b".to_owned(),
        )
        .expect("node checkpoint");
    let cost = history
        .replay_cost(Some(&entry_id("entry:c")))
        .expect("replay cost");
    assert_eq!(cost.checkpoint_id(), Some(&checkpoint_id("checkpoint:b")));
    assert_eq!(cost.entry_count(), 1);
    assert_eq!(cost.encoded_weight(), 8);
    assert_eq!(
        history
            .checkpoints()
            .find(|checkpoint| checkpoint.checkpoint_id() == &checkpoint_id("checkpoint:b"))
            .expect("checkpoint")
            .consumer_reference(),
        "snapshot://b"
    );

    let before = history.clone();
    assert_eq!(
        history.register_checkpoint(
            history.revision(),
            checkpoint_id("checkpoint:empty"),
            None,
            String::new(),
        ),
        Err(ForkCheckpointError::EmptyReference)
    );
    assert_eq!(history, before);
}

#[test]
fn structural_import_rejects_checkpoint_to_unknown_node() {
    let history = history();
    let state = history.into_state();
    let checkpoint = ForkCheckpoint::new(
        checkpoint_id("checkpoint:missing"),
        Some(entry_id("entry:missing")),
        "snapshot://missing".to_owned(),
    )
    .expect("bounded checkpoint");
    let imported = ForkHistoryState::new(
        state.history_id().clone(),
        state.revision(),
        state.current_branch_id().clone(),
        state.current_node_id().cloned(),
        state.next_sequence(),
    )
    .with_nodes(state.nodes().to_vec())
    .with_branches(state.branches().to_vec())
    .with_preferred_children(state.preferred_children().to_vec())
    .with_checkpoints(vec![checkpoint]);
    assert_eq!(
        ForkHistory::from_state(imported),
        Err(ForkHistoryStateError::InvalidCheckpoint(checkpoint_id(
            "checkpoint:missing"
        )))
    );
}

#[test]
fn deep_lineage_planning_is_iterative_and_exactly_bounded() {
    let mut history = history();
    let mut model = 0;
    for index in 0..2_048 {
        record(
            &mut history,
            &mut model,
            &format!("entry:depth-{index:04}"),
            1,
            None,
        );
    }
    let state = history.into_state();
    let root_position = ForkHistoryState::new(
        state.history_id().clone(),
        state.revision(),
        state.current_branch_id().clone(),
        None,
        state.next_sequence(),
    )
    .with_nodes(state.nodes().to_vec())
    .with_branches(state.branches().to_vec())
    .with_preferred_children(state.preferred_children().to_vec());
    let history = ForkHistory::from_state(root_position).expect("valid root position");
    let plan = history
        .plan_navigation(
            plan_id("plan:deep"),
            history.revision(),
            ForkNavigationTarget::Checkout {
                branch_id: branch_id("branch:main"),
                entry_id: entry_id("entry:depth-2047"),
            },
            &DeltaPolicy,
        )
        .expect("deep iterative plan");
    assert_eq!(plan.steps().len(), 2_048);
    assert!(
        plan.steps()
            .iter()
            .all(|step| matches!(step, HistoryNavigationStep::Redo { .. }))
    );
}

#[test]
fn stale_checkpoint_and_retention_requests_are_side_effect_free() {
    let mut history = history();
    let mut model = 0;
    record(&mut history, &mut model, "entry:a", 1, None);
    let before = history.clone();
    assert!(matches!(
        history.register_checkpoint(
            HistoryRevision::INITIAL,
            checkpoint_id("checkpoint:stale"),
            None,
            "snapshot://stale".to_owned()
        ),
        Err(ForkCheckpointError::StaleRevision { .. })
    ));
    assert!(matches!(
        history.prune_to(
            HistoryRevision::INITIAL,
            ForkRetentionLimits::new(1, 8).expect("fixture limits")
        ),
        Err(ForkRetentionError::StaleRevision { .. })
    ));
    assert_eq!(history, before);
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LoopholePulseMutation {
    route_id: String,
    track_index: usize,
    before: i64,
    after: i64,
    cache_epoch: u64,
}

struct PulsePolicy;

impl HistoryPolicy<LoopholePulseMutation> for PulsePolicy {
    type Error = &'static str;

    fn inverse(
        &self,
        payload: &LoopholePulseMutation,
    ) -> Result<LoopholePulseMutation, Self::Error> {
        Ok(LoopholePulseMutation {
            route_id: payload.route_id.clone(),
            track_index: payload.track_index,
            before: payload.after,
            after: payload.before,
            cache_epoch: payload.cache_epoch,
        })
    }

    fn is_noop(&self, payload: &LoopholePulseMutation) -> bool {
        payload.before == payload.after
    }

    fn encoded_weight(&self, _payload: &LoopholePulseMutation) -> Result<u64, Self::Error> {
        Ok(48)
    }

    fn coalesce(
        &self,
        _previous: &LoopholePulseMutation,
        _incoming: &LoopholePulseMutation,
        _context: HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<LoopholePulseMutation>, Self::Error> {
        Ok(HistoryCoalesce::KeepSeparate)
    }
}

struct PulseTransaction<'model> {
    tracks: &'model mut Vec<i64>,
    fail_after_first: bool,
}

impl ForkNavigationTransaction<LoopholePulseMutation> for PulseTransaction<'_> {
    type Error = &'static str;

    fn apply(
        &mut self,
        plan: &ForkNavigationPlan<LoopholePulseMutation>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        let source = self.tracks.clone();
        for (index, step) in plan.steps().iter().enumerate() {
            let payload = match step {
                HistoryNavigationStep::Undo { payload, .. }
                | HistoryNavigationStep::Redo { payload, .. } => payload,
            };
            if self.tracks.get(payload.track_index) != Some(&payload.before) {
                *self.tracks = source;
                return Err(HistoryNavigationTransactionFailure::RolledBack {
                    error: "stale Pulse model",
                });
            }
            self.tracks[payload.track_index] = payload.after;
            if self.fail_after_first && index == 0 {
                *self.tracks = source;
                return Err(HistoryNavigationTransactionFailure::RolledBack {
                    error: "Pulse reconciliation failed",
                });
            }
        }
        Ok(())
    }
}

fn pulse_record(
    history: &mut ForkHistory<LoopholePulseMutation>,
    tracks: &mut [i64],
    id: &str,
    track_index: usize,
    after: i64,
    divergent_branch: Option<ForkBranchSeed>,
) {
    let before = tracks[track_index];
    tracks[track_index] = after;
    history
        .record_applied(ForkRecord::new(
            history.revision(),
            entry_id(id),
            metadata(id),
            48,
            LoopholePulseMutation {
                route_id: format!("route:track-{track_index}"),
                track_index,
                before,
                after,
                cache_epoch: history.revision().get() + 1,
            },
            divergent_branch,
        ))
        .expect("Pulse fixture record");
}

#[test]
fn loophole_shaped_mixed_route_and_failure_invariance_keep_payload_semantics_external() {
    let mut history = ForkHistory::new(
        HistoryId::new("history:loophole-pulse").expect("fixture history id"),
        branch_id("branch:main"),
        branch_metadata(Some("Main"), true),
    );
    let mut tracks = vec![0, 0];
    pulse_record(&mut history, &mut tracks, "entry:a", 0, 1, None);
    pulse_record(&mut history, &mut tracks, "entry:b", 1, 2, None);
    pulse_record(&mut history, &mut tracks, "entry:c", 0, 3, None);

    let undo = history
        .plan_navigation(
            plan_id("plan:pulse-undo"),
            history.revision(),
            ForkNavigationTarget::Undo,
            &PulsePolicy,
        )
        .expect("Pulse undo plan");
    history
        .execute_navigation(
            undo,
            &mut PulseTransaction {
                tracks: &mut tracks,
                fail_after_first: false,
            },
        )
        .expect("Pulse undo");
    pulse_record(
        &mut history,
        &mut tracks,
        "entry:d",
        0,
        4,
        Some(ForkBranchSeed::new(
            branch_id("branch:alternate"),
            branch_metadata(Some("Alternate"), false),
        )),
    );
    assert_eq!(tracks, vec![4, 2]);

    let checkout = history
        .plan_navigation(
            plan_id("plan:pulse-main"),
            history.revision(),
            ForkNavigationTarget::Checkout {
                branch_id: branch_id("branch:main"),
                entry_id: entry_id("entry:c"),
            },
            &PulsePolicy,
        )
        .expect("Pulse mixed plan");
    let before_history = history.clone();
    let before_tracks = tracks.clone();
    assert_eq!(
        history.execute_navigation(
            checkout,
            &mut PulseTransaction {
                tracks: &mut tracks,
                fail_after_first: true,
            },
        ),
        Err(ForkNavigationError::RolledBack {
            error: "Pulse reconciliation failed"
        })
    );
    assert_eq!(history, before_history);
    assert_eq!(tracks, before_tracks);

    let checkout = history
        .plan_navigation(
            plan_id("plan:pulse-main-retry"),
            history.revision(),
            ForkNavigationTarget::Checkout {
                branch_id: branch_id("branch:main"),
                entry_id: entry_id("entry:c"),
            },
            &PulsePolicy,
        )
        .expect("Pulse retry plan");
    history
        .execute_navigation(
            checkout,
            &mut PulseTransaction {
                tracks: &mut tracks,
                fail_after_first: false,
            },
        )
        .expect("Pulse mixed commit");
    assert_eq!(tracks, vec![3, 2]);
    assert_eq!(history.current_branch_id(), &branch_id("branch:main"));
    assert_eq!(history.current_node_id(), Some(&entry_id("entry:c")));
}

/// A branch root is a real position: the state before that branch's first
/// entry, and where a nascent branch sits until something is recorded on it.
/// `Checkout` requires an entry id and so cannot name it, which forced
/// consumers to reach it by special-casing `AlreadyAtTarget` and
/// `UnknownTarget`. Card 181 step 3.
#[test]
fn checkout_branch_root_unwinds_to_the_position_before_the_first_entry() {
    let (mut history, mut model) = forked_history(branch_metadata(Some("Alternate"), false));
    let plan = history
        .plan_navigation(
            plan_id("plan:checkout-root"),
            history.revision(),
            ForkNavigationTarget::CheckoutBranchRoot {
                branch_id: branch_id("branch:main"),
            },
            &DeltaPolicy,
        )
        .expect("branch root plan");
    // Nothing is shared with a target that holds no entry, so the route is the
    // whole source lineage undone.
    assert_eq!(plan.lowest_common_ancestor(), None);
    assert!(
        plan.steps()
            .iter()
            .all(|step| matches!(step, HistoryNavigationStep::Undo { .. })),
        "a branch-root checkout only unwinds"
    );

    let mut transaction = ModelTransaction {
        model: &mut model,
        mode: TransactionMode::Commit,
        calls: 0,
    };
    let receipt = history
        .execute_navigation(plan, &mut transaction)
        .expect("branch root commit");
    assert_eq!(receipt.target_node_id(), None);
    assert_eq!(receipt.target_branch_id(), &branch_id("branch:main"));
    assert_eq!(model, 0, "unwinding every entry returns the model to zero");
}

#[test]
fn checkout_branch_root_rejects_a_branch_that_does_not_exist() {
    let (history, _model) = forked_history(branch_metadata(Some("Alternate"), false));
    let error = history
        .plan_navigation(
            plan_id("plan:checkout-missing"),
            history.revision(),
            ForkNavigationTarget::CheckoutBranchRoot {
                branch_id: branch_id("branch:absent"),
            },
            &DeltaPolicy,
        )
        .expect_err("a missing branch has no root to reach");
    assert!(
        matches!(error, ForkNavigationError::UnknownBranch(id) if id == branch_id("branch:absent"))
    );
}
