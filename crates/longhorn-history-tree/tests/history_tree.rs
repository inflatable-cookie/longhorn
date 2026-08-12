//! Fork-tree topology, invariant, document, and Loophole-shaped evidence.

use longhorn_core::{HistoryEntryId, HistoryId, HistoryKindId, HistoryRevision};
use longhorn_history::{
    HistoryEntryMetadata, HistoryEntrySequence, HistoryLabel, MAXIMUM_HISTORY_ENCODED_WEIGHT,
};
use longhorn_history_tree::{
    ForkBranch, ForkBranchId, ForkBranchMetadata, ForkBranchSeed, ForkHistory, ForkHistoryError,
    ForkHistoryNode, ForkHistoryState, ForkHistoryStateError, ForkPreferredChild, ForkRecord,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum DocumentMutation {
    Insert { offset: usize, text: String },
    Delete { offset: usize, text: String },
}

fn branch_id(value: &str) -> ForkBranchId {
    ForkBranchId::new(value).expect("fixture branch id")
}

fn entry_id(value: &str) -> HistoryEntryId {
    HistoryEntryId::new(value).expect("fixture entry id")
}

fn metadata(label: &str) -> HistoryEntryMetadata {
    HistoryEntryMetadata::new(
        HistoryLabel::new(label).expect("fixture label"),
        Some(HistoryKindId::new("fixture:document").expect("fixture kind")),
        None,
    )
}

fn branch_metadata(name: &str, pinned: bool) -> ForkBranchMetadata {
    ForkBranchMetadata::new(Some(name.to_owned()), None, pinned).expect("fixture branch metadata")
}

fn history() -> ForkHistory<DocumentMutation> {
    ForkHistory::new(
        HistoryId::new("history:document").expect("fixture history id"),
        branch_id("branch:main"),
        branch_metadata("Main", true),
    )
}

fn record(
    history: &mut ForkHistory<DocumentMutation>,
    id: &str,
    mutation: DocumentMutation,
    divergent_branch: Option<ForkBranchSeed>,
) {
    history
        .record_applied(ForkRecord::new(
            history.revision(),
            entry_id(id),
            metadata(id),
            16,
            mutation,
            divergent_branch,
        ))
        .expect("fixture record");
}

fn state_at(
    source: &ForkHistoryState<DocumentMutation>,
    current_branch_id: ForkBranchId,
    current_node_id: Option<HistoryEntryId>,
) -> ForkHistoryState<DocumentMutation> {
    ForkHistoryState::new(
        source.history_id().clone(),
        source.revision(),
        current_branch_id,
        current_node_id,
        source.next_sequence(),
    )
    .with_nodes(source.nodes().to_vec())
    .with_branches(source.branches().to_vec())
    .with_preferred_children(source.preferred_children().to_vec())
}

#[test]
fn branch_identity_metadata_and_payload_ownership_are_bounded() {
    assert!(ForkBranchId::new("").is_err());
    assert!(ForkBranchId::new("Branch:Main").is_err());
    assert!(ForkBranchId::new("b".repeat(129)).is_err());
    assert!(ForkBranchMetadata::new(Some(String::new()), None, false).is_err());
    assert!(ForkBranchMetadata::new(None, Some("a".repeat(4_097)), false).is_err());

    let mut graph = history();
    record(
        &mut graph,
        "entry:one",
        DocumentMutation::Insert {
            offset: 0,
            text: "hello".to_owned(),
        },
        None,
    );

    let node = graph.node(&entry_id("entry:one")).expect("retained node");
    assert_eq!(node.parent_entry_id(), None);
    assert!(matches!(
        node.payload(),
        DocumentMutation::Insert { offset: 0, text } if text == "hello"
    ));
    let main = graph
        .branch(&branch_id("branch:main"))
        .expect("main branch");
    assert_eq!(main.head_entry_id(), Some(&entry_id("entry:one")));
    assert_eq!(main.metadata().name(), Some("Main"));
}

#[test]
fn divergent_record_preserves_both_futures_and_exact_receipt() {
    let mut graph = history();
    for (id, offset, text) in [
        ("entry:a", 0, "a"),
        ("entry:b", 1, "b"),
        ("entry:c", 2, "c"),
    ] {
        record(
            &mut graph,
            id,
            DocumentMutation::Insert {
                offset,
                text: text.to_owned(),
            },
            None,
        );
    }

    let state = graph.into_state();
    let mut graph = ForkHistory::from_state(state_at(
        &state,
        branch_id("branch:main"),
        Some(entry_id("entry:b")),
    ))
    .expect("valid position behind branch head");
    let previous_revision = graph.revision();
    let receipt = graph
        .record_applied(ForkRecord::new(
            graph.revision(),
            entry_id("entry:d"),
            metadata("entry:d"),
            16,
            DocumentMutation::Delete {
                offset: 1,
                text: "b".to_owned(),
            },
            Some(ForkBranchSeed::new(
                branch_id("branch:alternate"),
                branch_metadata("Alternate", false),
            )),
        ))
        .expect("divergent record");

    assert_eq!(receipt.previous_revision(), previous_revision);
    assert_eq!(
        receipt.committed_revision().get(),
        previous_revision.get() + 1
    );
    assert_eq!(receipt.entry_id(), &entry_id("entry:d"));
    assert_eq!(receipt.branch_id(), &branch_id("branch:alternate"));
    assert_eq!(receipt.parent_entry_id(), Some(&entry_id("entry:b")));
    assert_eq!(receipt.previous_branch_head(), Some(&entry_id("entry:c")));
    assert_eq!(
        receipt.replaced_preferred_child(),
        Some(&entry_id("entry:c"))
    );
    assert!(receipt.diverged());
    assert_eq!(
        graph
            .branch(&branch_id("branch:main"))
            .expect("preserved main")
            .head_entry_id(),
        Some(&entry_id("entry:c"))
    );
    assert_eq!(
        graph
            .branch(&branch_id("branch:alternate"))
            .expect("new branch")
            .head_entry_id(),
        Some(&entry_id("entry:d"))
    );
    assert_eq!(
        graph.child_ids(Some(&entry_id("entry:b"))),
        &[entry_id("entry:c"), entry_id("entry:d")]
    );
    assert_eq!(
        graph.preferred_child_id(Some(&entry_id("entry:b"))),
        Some(&entry_id("entry:d"))
    );
    assert!(graph.node(&entry_id("entry:c")).is_some());
    assert!(graph.node(&entry_id("entry:d")).is_some());
}

#[test]
fn branch_reference_survives_head_advance_and_metadata_has_exact_receipt() {
    let mut graph = history();
    let stable_id = branch_id("branch:main");
    record(
        &mut graph,
        "entry:a",
        DocumentMutation::Insert {
            offset: 0,
            text: "a".to_owned(),
        },
        None,
    );
    record(
        &mut graph,
        "entry:b",
        DocumentMutation::Insert {
            offset: 1,
            text: "b".to_owned(),
        },
        None,
    );
    assert_eq!(
        graph
            .branch(&stable_id)
            .expect("stable ref")
            .head_entry_id(),
        Some(&entry_id("entry:b"))
    );

    let previous_revision = graph.revision();
    let changed = branch_metadata("Primary", false);
    let receipt = graph
        .set_branch_metadata(previous_revision, &stable_id, changed.clone())
        .expect("metadata replacement");
    assert_eq!(receipt.previous_revision(), previous_revision);
    assert_eq!(receipt.branch_id(), &stable_id);
    assert_eq!(receipt.previous_metadata().name(), Some("Main"));
    assert_eq!(receipt.committed_metadata(), &changed);
    assert_eq!(
        graph
            .branch(&stable_id)
            .expect("stable ref")
            .head_entry_id(),
        Some(&entry_id("entry:b"))
    );
}

#[test]
fn failed_records_leave_authority_exactly_unchanged() {
    let mut graph = history();
    record(
        &mut graph,
        "entry:a",
        DocumentMutation::Insert {
            offset: 0,
            text: "a".to_owned(),
        },
        None,
    );
    let before = graph.clone();
    let error = graph
        .record_applied(ForkRecord::new(
            HistoryRevision::INITIAL,
            entry_id("entry:b"),
            metadata("stale"),
            16,
            DocumentMutation::Insert {
                offset: 1,
                text: "b".to_owned(),
            },
            None,
        ))
        .expect_err("stale record");
    assert!(matches!(error, ForkHistoryError::StaleRevision { .. }));
    assert_eq!(graph, before);

    let state = graph.into_state();
    let mut graph = ForkHistory::from_state(state_at(&state, branch_id("branch:main"), None))
        .expect("valid root position");
    let before = graph.clone();
    let error = graph
        .record_applied(ForkRecord::new(
            graph.revision(),
            entry_id("entry:alternate"),
            metadata("alternate"),
            16,
            DocumentMutation::Insert {
                offset: 0,
                text: "alternate".to_owned(),
            },
            None,
        ))
        .expect_err("missing divergent branch seed");
    assert_eq!(error, ForkHistoryError::DivergentBranchRequired);
    assert_eq!(graph, before);
}

fn node(
    id: &str,
    parent: Option<&str>,
    sequence: u64,
    revision: u64,
) -> ForkHistoryNode<DocumentMutation> {
    ForkHistoryNode::new(
        entry_id(id),
        parent.map(entry_id),
        metadata(id),
        HistoryEntrySequence::new(sequence).expect("fixture sequence"),
        HistoryRevision::new(revision),
        16,
        DocumentMutation::Insert {
            offset: sequence as usize,
            text: id.to_owned(),
        },
    )
}

fn weighted_node(id: &str, encoded_weight: u64) -> ForkHistoryNode<DocumentMutation> {
    ForkHistoryNode::new(
        entry_id(id),
        None,
        metadata(id),
        HistoryEntrySequence::FIRST,
        HistoryRevision::new(1),
        encoded_weight,
        DocumentMutation::Insert {
            offset: 0,
            text: id.to_owned(),
        },
    )
}

fn imported_state(
    nodes: Vec<ForkHistoryNode<DocumentMutation>>,
    branches: Vec<ForkBranch>,
    current: Option<&str>,
    preferred: Vec<ForkPreferredChild>,
    next_sequence: u64,
) -> ForkHistoryState<DocumentMutation> {
    ForkHistoryState::new(
        HistoryId::new("history:import").expect("fixture history id"),
        HistoryRevision::new(10),
        branch_id("branch:main"),
        current.map(entry_id),
        HistoryEntrySequence::new(next_sequence).expect("fixture sequence"),
    )
    .with_nodes(nodes)
    .with_branches(branches)
    .with_preferred_children(preferred)
}

fn main_branch(head: Option<&str>) -> ForkBranch {
    ForkBranch::new(
        branch_id("branch:main"),
        head.map(entry_id),
        branch_metadata("Main", true),
    )
}

#[test]
fn malformed_topology_rejection_matrix_is_deterministic() {
    let cases = [
        (
            imported_state(vec![], vec![], None, vec![], 1),
            ForkHistoryStateError::MissingBranch,
        ),
        (
            imported_state(
                vec![node("entry:a", None, 1, 1), node("entry:a", None, 2, 2)],
                vec![main_branch(Some("entry:a"))],
                Some("entry:a"),
                vec![],
                3,
            ),
            ForkHistoryStateError::DuplicateNode(entry_id("entry:a")),
        ),
        (
            imported_state(
                vec![node("entry:a", None, 1, 1), node("entry:b", None, 1, 2)],
                vec![main_branch(Some("entry:b"))],
                Some("entry:b"),
                vec![],
                2,
            ),
            ForkHistoryStateError::DuplicateSequence(1),
        ),
        (
            imported_state(
                vec![node("entry:a", None, 1, 1), node("entry:b", None, 2, 1)],
                vec![main_branch(Some("entry:b"))],
                Some("entry:b"),
                vec![],
                3,
            ),
            ForkHistoryStateError::DuplicateCommittedRevision(1),
        ),
        (
            imported_state(
                vec![node("entry:a", None, 1, 0)],
                vec![main_branch(Some("entry:a"))],
                Some("entry:a"),
                vec![],
                2,
            ),
            ForkHistoryStateError::InvalidCommittedRevision(entry_id("entry:a")),
        ),
        (
            imported_state(
                vec![node("entry:b", Some("entry:missing"), 2, 2)],
                vec![main_branch(Some("entry:b"))],
                Some("entry:b"),
                vec![],
                3,
            ),
            ForkHistoryStateError::InvalidParent(entry_id("entry:b")),
        ),
        (
            imported_state(
                vec![
                    node("entry:a", None, 2, 2),
                    node("entry:b", Some("entry:a"), 1, 3),
                ],
                vec![main_branch(Some("entry:b"))],
                Some("entry:b"),
                vec![],
                3,
            ),
            ForkHistoryStateError::InvalidParent(entry_id("entry:b")),
        ),
        (
            imported_state(
                vec![node("entry:a", None, 1, 1)],
                vec![main_branch(Some("entry:missing"))],
                Some("entry:a"),
                vec![],
                2,
            ),
            ForkHistoryStateError::InvalidBranchHead(branch_id("branch:main")),
        ),
        (
            imported_state(
                vec![node("entry:a", None, 1, 1)],
                vec![main_branch(Some("entry:a")), main_branch(Some("entry:a"))],
                Some("entry:a"),
                vec![],
                2,
            ),
            ForkHistoryStateError::DuplicateBranch(branch_id("branch:main")),
        ),
        (
            imported_state(
                vec![node("entry:a", None, 1, 1), node("entry:b", None, 2, 2)],
                vec![main_branch(Some("entry:a"))],
                Some("entry:b"),
                // Two roots is a real choice, so the state has to name one.
                // Without this the case is rejected as MissingPreferredChild
                // and never reaches the current-node check it exists to test.
                vec![ForkPreferredChild::new(None, entry_id("entry:a"))],
                3,
            ),
            ForkHistoryStateError::InvalidCurrentNode,
        ),
        (
            imported_state(
                vec![node("entry:a", None, 1, 1), node("entry:b", None, 2, 2)],
                vec![main_branch(Some("entry:a"))],
                Some("entry:a"),
                vec![],
                3,
            ),
            ForkHistoryStateError::MissingPreferredChild(None),
        ),
        (
            imported_state(
                vec![node("entry:a", None, 1, 1), node("entry:b", None, 2, 2)],
                vec![main_branch(Some("entry:b"))],
                Some("entry:b"),
                vec![ForkPreferredChild::new(
                    Some(entry_id("entry:a")),
                    entry_id("entry:b"),
                )],
                3,
            ),
            ForkHistoryStateError::InvalidPreferredChild(entry_id("entry:b")),
        ),
        (
            imported_state(
                vec![node("entry:a", None, 1, 1), node("entry:b", None, 2, 2)],
                vec![main_branch(Some("entry:b"))],
                Some("entry:b"),
                vec![
                    ForkPreferredChild::new(None, entry_id("entry:a")),
                    ForkPreferredChild::new(None, entry_id("entry:b")),
                ],
                3,
            ),
            ForkHistoryStateError::DuplicatePreferredParent,
        ),
        (
            imported_state(
                vec![node("entry:a", None, 1, 1)],
                vec![main_branch(Some("entry:a"))],
                Some("entry:a"),
                vec![],
                1,
            ),
            ForkHistoryStateError::InvalidNextSequence,
        ),
        (
            imported_state(
                vec![weighted_node("entry:a", MAXIMUM_HISTORY_ENCODED_WEIGHT + 1)],
                vec![main_branch(Some("entry:a"))],
                Some("entry:a"),
                vec![],
                2,
            ),
            ForkHistoryStateError::InvalidEncodedWeight,
        ),
    ];

    for (state, expected) in cases {
        assert_eq!(ForkHistory::from_state(state), Err(expected));
    }

    let unknown_current_branch = ForkHistoryState::new(
        HistoryId::new("history:import").expect("fixture history id"),
        HistoryRevision::new(1),
        branch_id("branch:missing"),
        Some(entry_id("entry:a")),
        HistoryEntrySequence::new(2).expect("fixture sequence"),
    )
    .with_nodes(vec![node("entry:a", None, 1, 1)])
    .with_branches(vec![main_branch(Some("entry:a"))]);
    assert_eq!(
        ForkHistory::from_state(unknown_current_branch),
        Err(ForkHistoryStateError::UnknownCurrentBranch(branch_id(
            "branch:missing"
        )))
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LoopholePulseMutation {
    route_id: String,
    before: Vec<i16>,
    after: Vec<i16>,
    fork_origin: Option<HistoryEntryId>,
}

#[test]
fn loophole_shaped_fixture_keeps_deep_typed_futures_losslessly() {
    let mut graph = ForkHistory::new(
        HistoryId::new("history:loophole-session").expect("fixture history id"),
        branch_id("branch:main"),
        branch_metadata("Main", true),
    );
    for index in 0..96 {
        let id = format!("entry:pulse-{index:03}");
        graph
            .record_applied(ForkRecord::new(
                graph.revision(),
                entry_id(&id),
                metadata(&format!("Pulse edit {index}")),
                64,
                LoopholePulseMutation {
                    route_id: format!("route:bus-{}", index % 8),
                    before: vec![index - 1; 8],
                    after: vec![index; 8],
                    fork_origin: None,
                },
                None,
            ))
            .expect("main pulse record");
    }

    let state = graph.into_state();
    let fork_origin = entry_id("entry:pulse-047");
    let import = ForkHistoryState::new(
        state.history_id().clone(),
        state.revision(),
        branch_id("branch:main"),
        Some(fork_origin.clone()),
        state.next_sequence(),
    )
    .with_nodes(state.nodes().to_vec())
    .with_branches(state.branches().to_vec())
    .with_preferred_children(state.preferred_children().to_vec());
    let mut graph = ForkHistory::from_state(import).expect("valid Loophole position");
    let alternate_id = entry_id("entry:pulse-alternate");
    graph
        .record_applied(ForkRecord::new(
            graph.revision(),
            alternate_id.clone(),
            metadata("Alternate pulse edit"),
            64,
            LoopholePulseMutation {
                route_id: "route:bus-7".to_owned(),
                before: vec![47; 8],
                after: vec![-47; 8],
                fork_origin: Some(fork_origin.clone()),
            },
            Some(ForkBranchSeed::new(
                branch_id("branch:alternate"),
                branch_metadata("Alternate", true),
            )),
        ))
        .expect("alternate pulse record");

    assert_eq!(graph.retained_entry_count(), 97);
    assert_eq!(graph.retained_encoded_weight(), 97 * 64);
    assert!(graph.node(&entry_id("entry:pulse-095")).is_some());
    let alternate = graph.node(&alternate_id).expect("alternate pulse");
    assert_eq!(alternate.parent_entry_id(), Some(&fork_origin));
    assert_eq!(alternate.payload().fork_origin, Some(fork_origin));
    assert_eq!(
        graph.child_ids(Some(&entry_id("entry:pulse-047"))),
        &[entry_id("entry:pulse-048"), alternate_id]
    );
}
