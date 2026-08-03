//! Linear-default and explicit bounded alternate projection evidence.

use longhorn_core::{HistoryEntryId, HistoryId, HistoryKindId};
use longhorn_history::{
    HistoryEntryMetadata, HistoryEntryPosition, HistoryLabel, MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE,
};
use longhorn_history_tree::{
    ForkBranchId, ForkBranchMetadata, ForkBranchSeed, ForkHistory, ForkHistoryState,
    ForkProjectionError, ForkProjectionPageRequest, ForkRecord,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Mutation(i32);

fn history_id() -> HistoryId {
    HistoryId::new("history:projection").unwrap()
}

fn entry_id(value: &str) -> HistoryEntryId {
    HistoryEntryId::new(value).unwrap()
}

fn branch_id(value: &str) -> ForkBranchId {
    ForkBranchId::new(value).unwrap()
}

fn metadata(label: &str) -> HistoryEntryMetadata {
    HistoryEntryMetadata::new(
        HistoryLabel::new(label).unwrap(),
        Some(HistoryKindId::new("fixture:projection").unwrap()),
        None,
    )
}

fn branch_metadata(name: &str) -> ForkBranchMetadata {
    ForkBranchMetadata::new(Some(name.to_owned()), None, false).unwrap()
}

fn record(
    graph: &mut ForkHistory<Mutation>,
    id: &str,
    value: i32,
    divergent: Option<ForkBranchSeed>,
) {
    graph
        .record_applied(ForkRecord::new(
            graph.revision(),
            entry_id(id),
            metadata(id),
            4,
            Mutation(value),
            divergent,
        ))
        .unwrap();
}

fn forked_graph() -> ForkHistory<Mutation> {
    let mut graph = ForkHistory::new(
        history_id(),
        branch_id("branch:main"),
        branch_metadata("Main"),
    );
    record(&mut graph, "entry:a", 1, None);
    record(&mut graph, "entry:b", 2, None);
    record(&mut graph, "entry:c", 3, None);
    let state = graph.into_state();
    let mut graph = ForkHistory::from_state(
        ForkHistoryState::new(
            state.history_id().clone(),
            state.revision(),
            branch_id("branch:main"),
            Some(entry_id("entry:b")),
            state.next_sequence(),
        )
        .with_nodes(state.nodes().to_vec())
        .with_branches(state.branches().to_vec())
        .with_preferred_children(state.preferred_children().to_vec()),
    )
    .unwrap();
    record(
        &mut graph,
        "entry:d",
        4,
        Some(ForkBranchSeed::new(
            branch_id("branch:alternate"),
            branch_metadata("Alternate"),
        )),
    );
    graph
}

#[test]
fn default_summary_and_path_follow_preferred_redo_without_alternate_lists() {
    let graph = forked_graph();
    let summary = graph.project_summary().unwrap();
    assert_eq!(summary.history_id(), graph.history_id());
    assert_eq!(summary.revision(), graph.revision());
    assert_eq!(summary.current_branch_id(), &branch_id("branch:alternate"));
    assert_eq!(summary.current_entry_id(), Some(&entry_id("entry:d")));
    assert_eq!(summary.undo_depth(), 3);
    assert_eq!(summary.redo_depth(), 0);
    assert_eq!(summary.next_undo_label().unwrap().as_str(), "entry:d");
    assert_eq!(summary.next_redo_label(), None);
    assert_eq!(summary.retained_entry_count(), 4);
    assert_eq!(summary.retained_encoded_weight(), 16);
    assert_eq!(summary.branch_count(), 2);
    assert_eq!(summary.alternate_path_count(), 2);

    let page = graph
        .project_default_path_page(ForkProjectionPageRequest::new(0, 2).unwrap())
        .unwrap();
    assert_eq!(page.branch_id(), None);
    assert_eq!(page.head_entry_id(), Some(&entry_id("entry:d")));
    assert_eq!(page.total_entries(), 3);
    assert_eq!(page.entries().len(), 2);
    assert_eq!(page.entries()[0].entry_id(), &entry_id("entry:d"));
    assert_eq!(page.entries()[0].position(), HistoryEntryPosition::Current);
    assert_eq!(page.entries()[1].entry_id(), &entry_id("entry:b"));
    assert_eq!(page.entries()[1].position(), HistoryEntryPosition::Past);
    assert!(!page.truncated_before());
    assert!(page.truncated_after());
}

#[test]
fn branch_and_path_pages_are_explicit_stable_and_hard_bounded() {
    let graph = forked_graph();
    let first = graph
        .project_branch_page(ForkProjectionPageRequest::new(0, 1).unwrap())
        .unwrap();
    assert_eq!(first.total_branches(), 2);
    assert_eq!(first.branches().len(), 1);
    assert_eq!(
        first.branches()[0].branch_id(),
        &branch_id("branch:alternate")
    );
    assert_eq!(
        first.branches()[0].head_entry_id(),
        Some(&entry_id("entry:d"))
    );
    assert_eq!(
        first.branches()[0].divergence_entry_id(),
        Some(&entry_id("entry:d"))
    );
    assert!(first.branches()[0].current());
    assert!(first.truncated_after());

    let second = graph
        .project_branch_page(ForkProjectionPageRequest::new(1, 1).unwrap())
        .unwrap();
    assert!(second.truncated_before());
    assert!(!second.truncated_after());
    assert_eq!(second.branches()[0].branch_id(), &branch_id("branch:main"));
    assert_eq!(
        second.branches()[0].divergence_entry_id(),
        Some(&entry_id("entry:b"))
    );

    let main = graph
        .project_branch_path_page(
            &branch_id("branch:main"),
            ForkProjectionPageRequest::new(0, 4).unwrap(),
        )
        .unwrap();
    assert_eq!(main.branch_id(), Some(&branch_id("branch:main")));
    assert_eq!(main.head_entry_id(), Some(&entry_id("entry:c")));
    assert_eq!(main.entries()[0].entry_id(), &entry_id("entry:c"));
    assert_eq!(main.entries()[0].position(), HistoryEntryPosition::Future);
    assert_eq!(main.entries()[1].entry_id(), &entry_id("entry:b"));
    assert_eq!(main.entries()[1].position(), HistoryEntryPosition::Past);

    assert!(matches!(
        ForkProjectionPageRequest::new(0, 0),
        Err(ForkProjectionError::ZeroPageSize)
    ));
    assert!(matches!(
        ForkProjectionPageRequest::new(0, MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE + 1),
        Err(ForkProjectionError::PageTooLarge { .. })
    ));
    assert!(matches!(
        graph.project_branch_page(ForkProjectionPageRequest::new(3, 1).unwrap()),
        Err(ForkProjectionError::OffsetOutOfRange { .. })
    ));
    assert!(matches!(
        graph.project_branch_path_page(
            &branch_id("branch:missing"),
            ForkProjectionPageRequest::new(0, 1).unwrap()
        ),
        Err(ForkProjectionError::UnknownBranch(_))
    ));
}

#[test]
fn deep_wide_shape_returns_only_the_requested_records() {
    let mut graph = ForkHistory::new(
        HistoryId::new("history:deep-projection").unwrap(),
        branch_id("branch:main"),
        branch_metadata("Main"),
    );
    for index in 1..=2_048 {
        record(&mut graph, &format!("entry:main-{index:04}"), index, None);
    }

    let summary = graph.project_summary().unwrap();
    assert_eq!(summary.retained_entry_count(), 2_048);
    assert_eq!(summary.branch_count(), 1);
    let page = graph
        .project_default_path_page(ForkProjectionPageRequest::new(1_000, 17).unwrap())
        .unwrap();
    assert_eq!(page.total_entries(), 2_048);
    assert_eq!(page.entries().len(), 17);
    assert!(page.truncated_before());
    assert!(page.truncated_after());
}
