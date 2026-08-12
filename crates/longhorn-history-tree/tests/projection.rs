//! Linear-default and explicit bounded alternate projection evidence.

use longhorn_core::{HistoryEntryId, HistoryId, HistoryKindId};
use longhorn_history::{
    HistoryAuthorityEpoch, HistoryEntryMetadata, HistoryEntryPosition, HistoryLabel,
    HistoryRecordedAt,
};
use longhorn_history_tree::{
    ForkBranchId, ForkBranchMetadata, ForkBranchSeed, ForkHistory, ForkHistoryState,
    ForkHistoryStateError, ForkPathPageSnapshot, ForkProjectionError, ForkProjectionPageRequest,
    ForkRecord, MAXIMUM_FORK_PROJECTION_PAGE_SIZE,
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

/// Epoch milliseconds a host would have stamped. Card 182: the tree never
/// reads this, so the fixture value only has to be recognisable downstream.
const FIXTURE_RECORDED_AT: u64 = 1_765_432_100_000;

fn metadata(label: &str) -> HistoryEntryMetadata {
    HistoryEntryMetadata::new(
        HistoryLabel::new(label).unwrap(),
        Some(HistoryKindId::new("fixture:projection").unwrap()),
        None,
    )
    .with_recorded_at(HistoryRecordedAt::from_epoch_millis(FIXTURE_RECORDED_AT))
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
    // Card 183: `entry:b`, not `entry:d`. The old computation zipped a branch
    // against the current branch -- which here is `alternate` itself, so it
    // reported alternate's own head as its divergence. Divergence is now
    // relative to the nearest ancestor branch, which is `main` at `entry:b`.
    assert_eq!(
        first.branches()[0].divergence_entry_id(),
        Some(&entry_id("entry:b"))
    );
    assert_eq!(
        first.branches()[0].divergence_branch_id(),
        Some(&branch_id("branch:main"))
    );
    assert!(first.branches()[0].current());
    assert!(first.truncated_after());

    let second = graph
        .project_branch_page(ForkProjectionPageRequest::new(1, 1).unwrap())
        .unwrap();
    assert!(second.truncated_before());
    assert!(!second.truncated_after());
    assert_eq!(second.branches()[0].branch_id(), &branch_id("branch:main"));
    // Card 183: `None`. `main` is the run everything else forked off, so it
    // diverges from nothing. The old computation reported `entry:b` only
    // because it zipped main against the current branch, which had forked
    // there -- a fact about `alternate`, reported on `main`.
    assert_eq!(second.branches()[0].divergence_entry_id(), None);
    assert_eq!(second.branches()[0].divergence_branch_id(), None);

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
        ForkProjectionPageRequest::new(0, MAXIMUM_FORK_PROJECTION_PAGE_SIZE + 1),
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

/// Card 182. A host-supplied stamp survives projection and reaches
/// `ForkEntryRecord`, which is the only reason the field exists: version
/// captions read it from there.
#[test]
fn a_recorded_at_stamp_reaches_the_entry_record() {
    let graph = forked_graph();
    let page = graph
        .project_default_path_page(ForkProjectionPageRequest::new(0, 8).unwrap())
        .unwrap();
    assert!(!page.entries().is_empty());

    let stamp = Some(HistoryRecordedAt::from_epoch_millis(FIXTURE_RECORDED_AT));
    assert!(
        page.entries()
            .iter()
            .all(|entry| entry.recorded_at() == stamp),
        "every projected entry carries the stamp its metadata held"
    );

    let snapshot =
        ForkPathPageSnapshot::from_page(HistoryAuthorityEpoch::new(1).unwrap(), &page).unwrap();
    assert!(
        snapshot
            .entries
            .iter()
            .all(|record| record.recorded_at == stamp),
        "and the wire record carries it too"
    );
}

/// Rewinds to `at` on `branch`, then records `id` onto a fresh branch.
/// The state rebuild is how the other fixtures here move the cursor without a
/// policy, and this keeps that.
fn fork_at(
    graph: ForkHistory<Mutation>,
    at: &str,
    branch: &str,
    id: &str,
    value: i32,
    seed: (&str, &str),
) -> ForkHistory<Mutation> {
    let state = graph.into_state();
    let mut graph = ForkHistory::from_state(
        ForkHistoryState::new(
            state.history_id().clone(),
            state.revision(),
            branch_id(branch),
            Some(entry_id(at)),
            state.next_sequence(),
        )
        .with_nodes(state.nodes().to_vec())
        .with_branches(state.branches().to_vec())
        .with_preferred_children(state.preferred_children().to_vec()),
    )
    .unwrap();
    record(
        &mut graph,
        id,
        value,
        Some(ForkBranchSeed::new(
            branch_id(seed.0),
            branch_metadata(seed.1),
        )),
    );
    graph
}

/// a - b - c            (main)
///       \ d            (alternate)
///       \ e            (third)
fn two_forks_at_one_entry() -> ForkHistory<Mutation> {
    fork_at(
        forked_graph(),
        "entry:b",
        "branch:main",
        "entry:e",
        5,
        ("branch:third", "Third"),
    )
}

/// Card 183. Two forks at one entry is the case v2's row list could not
/// distinguish from a fork off a fork; the count and the page both see three.
#[test]
fn one_entry_carries_every_continuation_it_has() {
    let graph = two_forks_at_one_entry();
    let page = graph
        .project_branch_path_page(
            &branch_id("branch:main"),
            ForkProjectionPageRequest::new(0, 8).unwrap(),
        )
        .unwrap();
    let b = page
        .entries()
        .iter()
        .find(|entry| entry.entry_id() == &entry_id("entry:b"))
        .expect("entry:b is on main");
    assert_eq!(b.continuation_count(), 3, "c, d and e all continue from b");

    let continuations = graph
        .project_continuations(
            Some(&entry_id("entry:b")),
            ForkProjectionPageRequest::new(0, 8).unwrap(),
        )
        .unwrap();
    assert_eq!(continuations.total_continuations(), 3);
    let ids: Vec<_> = continuations
        .continuations()
        .iter()
        .map(|continuation| continuation.entry_id().as_str().to_owned())
        .collect();
    assert_eq!(ids, ["entry:c", "entry:d", "entry:e"]);
    assert_eq!(
        continuations
            .continuations()
            .iter()
            .filter(|continuation| continuation.preferred())
            .count(),
        1,
        "exactly one future is the one a redo takes"
    );
    assert!(
        continuations
            .continuations()
            .iter()
            .all(|continuation| continuation.entry_count() == 1),
        "each of these runs is a single entry"
    );
    assert_eq!(
        continuations
            .continuations()
            .iter()
            .find(|continuation| continuation.entry_id() == &entry_id("entry:e"))
            .map(|continuation| continuation.branch_name()),
        Some(Some("Third")),
        "a continuation names the branch taking it lands on"
    );
}

/// Card 183. The case the old current-branch computation collapsed: a fork off
/// a fork has to name its own parent run, not whatever the operator is reading.
#[test]
fn a_fork_off_a_fork_diverges_from_its_own_parent() {
    // a - b - c        (main)
    //       \ d        (alternate)
    //          \ f     (deep, off alternate)
    let mut extended = forked_graph();
    // alternate: a - b - d - g. Recording at d while d is alternate's head
    // would be a linear extension, and the graph rejects a seed for that.
    record(&mut extended, "entry:g", 7, None);
    let graph = fork_at(
        extended,
        "entry:d",
        "branch:alternate",
        "entry:f",
        6,
        ("branch:deep", "Deep"),
    );
    let page = graph
        .project_branch_page(ForkProjectionPageRequest::new(0, 8).unwrap())
        .unwrap();
    let of = |name: &str| {
        page.branches()
            .iter()
            .find(|branch| branch.branch_id() == &branch_id(name))
            .map(|branch| {
                (
                    branch.divergence_entry_id().cloned(),
                    branch.divergence_branch_id().cloned(),
                )
            })
            .expect("branch is on the page")
    };
    assert_eq!(
        of("branch:deep"),
        (
            Some(entry_id("entry:d")),
            Some(branch_id("branch:alternate"))
        ),
        "deep forked off alternate at d, not off main at b"
    );
    assert_eq!(
        of("branch:alternate").0,
        Some(entry_id("entry:b")),
        "alternate still forked off b"
    );
}

/// Card 183. The nested list is the root list. If these pages ever disagree,
/// a renderer cannot recurse into the same component.
#[test]
fn a_continuation_run_matches_the_path_page_for_the_same_entries() {
    let graph = two_forks_at_one_entry();
    let run = graph
        .project_continuation_run_page(
            &entry_id("entry:d"),
            ForkProjectionPageRequest::new(0, 8).unwrap(),
        )
        .unwrap();
    assert_eq!(run.total_entries(), 1);
    let entry = &run.entries()[0];
    assert_eq!(entry.entry_id(), &entry_id("entry:d"));
    assert_eq!(
        entry.continuation_count(),
        0,
        "a run's last entry has no children at all, so no forks either"
    );

    // The current branch is `third`, so `entry:e` is the default path's head.
    // Its record has to be identical whichever projection produced it.
    let run = graph
        .project_continuation_run_page(
            &entry_id("entry:e"),
            ForkProjectionPageRequest::new(0, 8).unwrap(),
        )
        .unwrap();
    let default = graph
        .project_default_path_page(ForkProjectionPageRequest::new(0, 8).unwrap())
        .unwrap();
    let from_default = default
        .entries()
        .iter()
        .find(|candidate| candidate.entry_id() == &entry_id("entry:e"))
        .expect("entry:e is the default path head");
    assert_eq!(&run.entries()[0], from_default, "same entry, same record");
    assert_eq!(
        default.preceding_continuation_count(),
        1,
        "one root, so no fork badge above the first entry"
    );

    // The same field on a run page is about the run's anchor, not the history
    // root. entry:e was recorded at entry:b, which has three continuations.
    let anchored = graph
        .project_continuation_run_page(
            &entry_id("entry:e"),
            ForkProjectionPageRequest::new(0, 8).unwrap(),
        )
        .unwrap();
    assert_eq!(
        anchored.preceding_continuation_count(),
        3,
        "the position above this run is entry:b, which has three continuations"
    );
}

/// Card 183 follow-up. Every forward walk in the crate follows preferred
/// children, so a node with children and no preference truncates all of them
/// and hides whatever is past it. Recording and pruning maintain the
/// preference; a hand-built state is the only way to omit it, and it is
/// rejected rather than left to surface as a fork nobody can open.
#[test]
fn a_state_whose_node_has_children_but_no_preference_is_rejected() {
    let state = forked_graph().into_state();
    let error = ForkHistory::<Mutation>::from_state(
        ForkHistoryState::new(
            state.history_id().clone(),
            state.revision(),
            branch_id("branch:alternate"),
            Some(entry_id("entry:d")),
            state.next_sequence(),
        )
        .with_nodes(state.nodes().to_vec())
        .with_branches(state.branches().to_vec()),
        // preferred_children deliberately omitted
    )
    .expect_err("a graph with children and no preference is not projectable");
    assert!(matches!(
        error,
        ForkHistoryStateError::MissingPreferredChild(_)
    ));
}
