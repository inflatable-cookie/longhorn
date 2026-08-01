//! Divergent topology, LCA checkout, and preferred-redo evidence.

mod support;

use longhorn_history::HistoryNavigationStep;
use longhorn_history_tree_prototype::{ForkBranchSeed, ForkNavigationTarget};

use support::{
    DeltaPolicy, ModelTransaction, TransactionMode, branch_id, branch_metadata, entry_id, history,
    plan_id, record,
};

#[test]
fn divergent_record_preserves_both_futures_and_stable_branch_refs() {
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
    let mut transaction = ModelTransaction {
        model: &mut model,
        mode: TransactionMode::Commit,
        calls: 0,
    };
    history.execute_navigation(undo, &mut transaction).unwrap();
    assert_eq!(model, 3);

    let alternate_id = branch_id("branch:alternate");
    let receipt = {
        let seed = ForkBranchSeed::new(alternate_id.clone(), branch_metadata("Alternate", true));
        record(&mut history, &mut model, "entry:d", 4, Some(seed));
        history.branch(&alternate_id).unwrap().clone()
    };

    assert_eq!(model, 7);
    assert_eq!(history.retained_entry_count(), 4);
    assert_eq!(
        history
            .branch(&branch_id("branch:main"))
            .unwrap()
            .head_entry_id(),
        Some(&entry_id("entry:c")),
    );
    assert_eq!(receipt.branch_id(), &alternate_id);
    assert_eq!(receipt.head_entry_id(), Some(&entry_id("entry:d")));
    assert_eq!(history.node(&entry_id("entry:c")).unwrap().payload().0, 3);
    assert_eq!(history.node(&entry_id("entry:d")).unwrap().payload().0, 4);
}

#[test]
fn lca_checkout_is_one_mixed_atomic_route_and_updates_preferred_redo() {
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

    let undo_d = history
        .plan_navigation(
            plan_id("plan:undo-d"),
            history.revision(),
            ForkNavigationTarget::Undo,
            &DeltaPolicy,
        )
        .unwrap();
    history
        .execute_navigation(
            undo_d,
            &mut ModelTransaction {
                model: &mut model,
                mode: TransactionMode::Commit,
                calls: 0,
            },
        )
        .unwrap();
    let initial_preferred = history
        .plan_navigation(
            plan_id("plan:initial-preferred"),
            history.revision(),
            ForkNavigationTarget::Redo,
            &DeltaPolicy,
        )
        .unwrap();
    assert_eq!(
        initial_preferred.target_branch_id(),
        &branch_id("branch:alternate"),
    );
    history
        .execute_navigation(
            initial_preferred,
            &mut ModelTransaction {
                model: &mut model,
                mode: TransactionMode::Commit,
                calls: 0,
            },
        )
        .unwrap();

    let checkout = history
        .plan_navigation(
            plan_id("plan:checkout-main"),
            history.revision(),
            ForkNavigationTarget::Checkout {
                branch_id: branch_id("branch:main"),
                entry_id: entry_id("entry:c"),
            },
            &DeltaPolicy,
        )
        .unwrap();
    assert_eq!(
        checkout.lowest_common_ancestor(),
        Some(&entry_id("entry:b")),
    );
    assert!(matches!(
        &checkout.steps()[0],
        HistoryNavigationStep::Undo { entry_id: id, payload }
            if id == &entry_id("entry:d") && payload.0 == -4
    ));
    assert!(matches!(
        &checkout.steps()[1],
        HistoryNavigationStep::Redo { entry_id: id, payload }
            if id == &entry_id("entry:c") && payload.0 == 3
    ));

    let mut transaction = ModelTransaction {
        model: &mut model,
        mode: TransactionMode::Commit,
        calls: 0,
    };
    let receipt = history
        .execute_navigation(checkout, &mut transaction)
        .unwrap();
    assert_eq!(transaction.calls, 1);
    assert_eq!(model, 6);
    assert_eq!(receipt.target_branch_id(), &branch_id("branch:main"));

    let checkout_b = history
        .plan_navigation(
            plan_id("plan:checkout-b"),
            history.revision(),
            ForkNavigationTarget::Checkout {
                branch_id: branch_id("branch:main"),
                entry_id: entry_id("entry:b"),
            },
            &DeltaPolicy,
        )
        .unwrap();
    history
        .execute_navigation(
            checkout_b,
            &mut ModelTransaction {
                model: &mut model,
                mode: TransactionMode::Commit,
                calls: 0,
            },
        )
        .unwrap();

    let redo = history
        .plan_navigation(
            plan_id("plan:preferred-redo"),
            history.revision(),
            ForkNavigationTarget::Redo,
            &DeltaPolicy,
        )
        .unwrap();
    assert_eq!(redo.target_branch_id(), &branch_id("branch:main"));
    assert_eq!(redo.target_node_id(), Some(&entry_id("entry:c")));
}
