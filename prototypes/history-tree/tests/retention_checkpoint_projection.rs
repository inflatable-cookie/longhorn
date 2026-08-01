//! Retention, opaque checkpoint, replay-cost, and projection evidence.

mod support;

use longhorn_history::HistoryEntryPosition;
use longhorn_history_tree_prototype::{
    ForkBranchMetadata, ForkBranchSeed, ForkCheckpointId, ForkNavigationTarget, ForkPruningOutcome,
    ForkRecord, ForkRetentionError, ForkRetentionLimits,
};

use support::{
    Delta, DeltaPolicy, ModelTransaction, TransactionMode, branch_id, branch_metadata, entry_id,
    history, metadata, plan_id, record,
};

fn divergent_history(
    alternate_metadata: ForkBranchMetadata,
) -> (longhorn_history_tree_prototype::ForkHistory<Delta>, i64) {
    let mut history = history();
    let mut model = 0;
    record(&mut history, &mut model, "entry:a", 1, None);
    record(&mut history, &mut model, "entry:b", 2, None);
    record(&mut history, &mut model, "entry:c", 3, None);
    let undo = history
        .plan_navigation(
            plan_id("plan:undo-c"),
            history.revision(),
            ForkNavigationTarget::Undo,
            &DeltaPolicy,
        )
        .unwrap();
    history
        .execute_navigation(
            undo,
            &mut ModelTransaction {
                model: &mut model,
                mode: TransactionMode::Commit,
                calls: 0,
            },
        )
        .unwrap();
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
fn nearest_opaque_checkpoint_bounds_replay_without_owning_content() {
    let (mut history, _) = divergent_history(
        ForkBranchMetadata::new(Some("Alternate".to_owned()), None, true).unwrap(),
    );
    history
        .register_checkpoint(
            history.revision(),
            ForkCheckpointId::new("checkpoint:root").unwrap(),
            None,
            "consumer://snapshot/root".to_owned(),
        )
        .unwrap();
    history
        .register_checkpoint(
            history.revision(),
            ForkCheckpointId::new("checkpoint:b").unwrap(),
            Some(entry_id("entry:b")),
            "consumer://snapshot/b".to_owned(),
        )
        .unwrap();

    let cost = history.replay_cost(Some(&entry_id("entry:d"))).unwrap();
    assert_eq!(cost.checkpoint_id().unwrap().as_str(), "checkpoint:b",);
    assert_eq!(cost.entry_count(), 1);
    assert_eq!(cost.encoded_weight(), 8);
    assert_eq!(
        history
            .checkpoints()
            .find(|checkpoint| checkpoint.checkpoint_id().as_str() == "checkpoint:b")
            .unwrap()
            .consumer_reference(),
        "consumer://snapshot/b",
    );
}

#[test]
fn pruning_removes_oldest_unprotected_leaf_branch_and_checkpoint() {
    let anonymous = ForkBranchMetadata::new(None, Some("scratch".to_owned()), false).unwrap();
    let (mut history, mut model) = divergent_history(anonymous);
    history
        .register_checkpoint(
            history.revision(),
            ForkCheckpointId::new("checkpoint:d").unwrap(),
            Some(entry_id("entry:d")),
            "consumer://snapshot/d".to_owned(),
        )
        .unwrap();
    let checkout = history
        .plan_navigation(
            plan_id("plan:return-main"),
            history.revision(),
            ForkNavigationTarget::Checkout {
                branch_id: branch_id("branch:main"),
                entry_id: entry_id("entry:c"),
            },
            &DeltaPolicy,
        )
        .unwrap();
    history
        .execute_navigation(
            checkout,
            &mut ModelTransaction {
                model: &mut model,
                mode: TransactionMode::Commit,
                calls: 0,
            },
        )
        .unwrap();

    let outcome = history
        .prune_to(history.revision(), ForkRetentionLimits::new(3, 24).unwrap())
        .unwrap();
    let ForkPruningOutcome::Pruned(receipt) = outcome else {
        panic!("one alternate node must prune");
    };
    assert_eq!(receipt.pruned_nodes().len(), 1);
    assert_eq!(receipt.pruned_nodes()[0].entry_id(), &entry_id("entry:d"));
    assert_eq!(receipt.removed_branches(), &[branch_id("branch:alternate")]);
    assert_eq!(receipt.removed_checkpoints()[0].as_str(), "checkpoint:d",);
    assert_eq!(receipt.retained_entry_count(), 3);
    assert_eq!(receipt.retained_encoded_weight(), 24);
    assert_eq!(history.current_node_id(), Some(&entry_id("entry:c")));
}

#[test]
fn impossible_budget_terminates_and_preserves_exact_graph() {
    let (mut history, _) = divergent_history(branch_metadata("Alternate", true));
    let source = history.clone();
    let error = history
        .prune_to(history.revision(), ForkRetentionLimits::new(2, 16).unwrap())
        .unwrap_err();
    assert!(matches!(
        error,
        ForkRetentionError::ProtectedBudget {
            protected_entries: 4,
            protected_encoded_weight: 32,
        }
    ));
    assert_eq!(history, source);
}

#[test]
fn default_projection_stays_linear_and_alternates_are_opt_in() {
    let (mut history, mut model) = divergent_history(branch_metadata("Alternate", false));
    let linear = history.linear_projection().unwrap();
    let ids: Vec<_> = linear
        .entries()
        .iter()
        .map(|entry| entry.entry_id().as_str())
        .collect();
    assert_eq!(ids, ["entry:a", "entry:b", "entry:d"]);
    assert_eq!(
        linear.entries()[2].position(),
        HistoryEntryPosition::Current,
    );

    let alternates = history.alternate_projection().unwrap();
    assert_eq!(alternates.branches().len(), 2);
    let leaves: Vec<_> = alternates
        .derived_paths()
        .iter()
        .map(|path| path.leaf_entry_id().as_str())
        .collect();
    assert_eq!(leaves, ["entry:c", "entry:d"]);

    let stable_branch_id = history.current_branch_id().clone();
    model += 5;
    history
        .record_applied(ForkRecord::new(
            history.revision(),
            entry_id("entry:e"),
            metadata("entry:e"),
            8,
            Delta(5),
            None,
        ))
        .unwrap();
    let advanced = history.alternate_projection().unwrap();
    let advanced_leaves: Vec<_> = advanced
        .derived_paths()
        .iter()
        .map(|path| path.leaf_entry_id().as_str())
        .collect();
    assert_eq!(advanced_leaves, ["entry:c", "entry:e"]);
    assert_eq!(history.current_branch_id(), &stable_branch_id);
    assert_eq!(
        history.branch(&stable_branch_id).unwrap().head_entry_id(),
        Some(&entry_id("entry:e")),
    );
    assert_eq!(model, 12);
}
