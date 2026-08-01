//! Atomic navigation failure-invariance evidence.

mod support;

use longhorn_history_tree_prototype::{ForkBranchSeed, ForkNavigationError, ForkNavigationTarget};

use support::{
    DeltaPolicy, ModelTransaction, TransactionMode, branch_id, branch_metadata, entry_id, history,
    plan_id, record,
};

fn divergent_history() -> (
    longhorn_history_tree_prototype::ForkHistory<support::Delta>,
    i64,
) {
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
            branch_metadata("Alternate", false),
        )),
    );
    (history, model)
}

#[test]
fn verified_rollback_preserves_exact_model_and_graph() {
    let (mut history, mut model) = divergent_history();
    let source_history = history.clone();
    let source_model = model;
    let plan = history
        .plan_navigation(
            plan_id("plan:rollback"),
            history.revision(),
            ForkNavigationTarget::Checkout {
                branch_id: branch_id("branch:main"),
                entry_id: entry_id("entry:c"),
            },
            &DeltaPolicy,
        )
        .unwrap();

    let error = history
        .execute_navigation(
            plan,
            &mut ModelTransaction {
                model: &mut model,
                mode: TransactionMode::RollBack,
                calls: 0,
            },
        )
        .unwrap_err();
    assert_eq!(
        error,
        ForkNavigationError::RolledBack {
            error: "apply failed"
        }
    );
    assert_eq!(history, source_history);
    assert_eq!(model, source_model);
}

#[test]
fn rollback_failure_preserves_graph_and_reports_partial_model() {
    let (mut history, mut model) = divergent_history();
    let source_history = history.clone();
    let source_model = model;
    let plan = history
        .plan_navigation(
            plan_id("plan:rollback-fails"),
            history.revision(),
            ForkNavigationTarget::Checkout {
                branch_id: branch_id("branch:main"),
                entry_id: entry_id("entry:c"),
            },
            &DeltaPolicy,
        )
        .unwrap();

    let error = history
        .execute_navigation(
            plan,
            &mut ModelTransaction {
                model: &mut model,
                mode: TransactionMode::RollbackFails,
                calls: 0,
            },
        )
        .unwrap_err();
    assert_eq!(
        error,
        ForkNavigationError::RollbackFailed {
            error: "apply failed",
            rollback_error: "rollback failed",
        },
    );
    assert_eq!(history, source_history);
    assert_ne!(model, source_model);
}

#[test]
fn stale_plan_rejects_before_product_transaction() {
    let (mut history, mut model) = divergent_history();
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
        .unwrap();
    history
        .set_branch_metadata(
            history.revision(),
            &branch_id("branch:main"),
            branch_metadata("Renamed Main", true),
        )
        .unwrap();
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
}
