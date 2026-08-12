use std::{convert::Infallible, time::Instant};

use longhorn_core::{
    HistoryEntryId, HistoryGroupId, HistoryId, HistoryKindId, HistoryPlanId, HistoryRevision,
};
use longhorn_history::{
    HistoryAuthorityEpoch, HistoryCoalesce, HistoryCoalesceContext, HistoryEntryMetadata,
    HistoryEntrySequence, HistoryLabel, HistoryNavigationTransactionFailure, HistoryPayloadCodec,
    HistoryPayloadCodecFamily, HistoryPayloadCodecVersion, HistoryPolicy,
};
use longhorn_history_tree::{
    ForkBranch, ForkBranchId, ForkBranchMetadata, ForkChangedEvent, ForkChangedKind,
    ForkCheckpointId, ForkHistory, ForkHistoryNode, ForkHistoryState,
    ForkNavigationReceiptProjection, ForkNavigationResult, ForkNavigationTarget,
    ForkNavigationTransaction, ForkPathPageSnapshot, ForkPersistence, ForkPersistenceLimits,
    ForkPreferredChild, ForkProjectionPageRequest, ForkRecord, ForkRetentionLimits, ForkSnapshot,
};
use serde_json::json;

const AUTHORITY_EPOCH: u64 = 7;
const PAGE_SIZE: usize = 17;

#[derive(Clone)]
struct DenseCodec {
    family: HistoryPayloadCodecFamily,
}

impl DenseCodec {
    fn new() -> Self {
        Self {
            family: HistoryPayloadCodecFamily::new("proof.fork-tree-payload")
                .expect("proof codec family"),
        }
    }
}

impl HistoryPayloadCodec<Vec<u8>> for DenseCodec {
    type Error = Infallible;

    fn family(&self) -> &HistoryPayloadCodecFamily {
        &self.family
    }

    fn version(&self) -> HistoryPayloadCodecVersion {
        HistoryPayloadCodecVersion::new(1)
    }

    fn encode(&self, payload: &Vec<u8>) -> Result<Vec<u8>, Self::Error> {
        Ok(payload.clone())
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, Self::Error> {
        Ok(bytes.to_vec())
    }
}

struct BytesPolicy;

impl HistoryPolicy<Vec<u8>> for BytesPolicy {
    type Error = Infallible;

    fn inverse(&self, payload: &Vec<u8>) -> Result<Vec<u8>, Self::Error> {
        Ok(payload.clone())
    }

    fn is_noop(&self, _payload: &Vec<u8>) -> bool {
        false
    }

    fn encoded_weight(&self, payload: &Vec<u8>) -> Result<u64, Self::Error> {
        Ok(payload.len() as u64)
    }

    fn coalesce(
        &self,
        _previous: &Vec<u8>,
        _incoming: &Vec<u8>,
        _context: HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<Vec<u8>>, Self::Error> {
        Ok(HistoryCoalesce::KeepSeparate)
    }
}

#[derive(Default)]
struct CountingTransaction {
    calls: usize,
    steps: usize,
}

impl ForkNavigationTransaction<Vec<u8>> for CountingTransaction {
    type Error = Infallible;

    fn apply(
        &mut self,
        plan: &longhorn_history_tree::ForkNavigationPlan<Vec<u8>>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        self.calls += 1;
        self.steps += plan.steps().len();
        Ok(())
    }
}

fn main() {
    let loophole = env!("CARGO_PKG_NAME").contains("loophole");
    let shape = if loophole { "loophole" } else { "document" };
    let (main_depth, alternates, payload_bytes, numeric_baseline) = if loophole {
        (2_048, 64, 256, 7_534_856)
    } else {
        (128, 4, 32, 99_295)
    };
    let base = shaped_graph(shape, main_depth, alternates, payload_bytes);
    let persistence = ForkPersistence::without_structural_migration(
        DenseCodec::new(),
        ForkPersistenceLimits::new(64 * 1_024 * 1_024).expect("proof persistence limit"),
    );

    let encode_started = Instant::now();
    let encoded = persistence.encode(&base).expect("dense encode");
    let encode_micros = encode_started.elapsed().as_micros();
    let load_started = Instant::now();
    let loaded = persistence
        .load(base.history_id(), &encoded)
        .expect("dense load");
    let load_micros = load_started.elapsed().as_micros();
    assert_eq!(loaded.history(), &base);

    let mut graph = base.clone();
    let target_branch = branch_id(&format!("branch:alternate-{alternates:02}"));
    let target_entry = entry_id(&format!("entry:alternate-{alternates:02}"));
    let plan = graph
        .plan_navigation(
            plan_id("plan:artifact-checkout"),
            graph.revision(),
            ForkNavigationTarget::Checkout {
                branch_id: target_branch,
                entry_id: target_entry,
            },
            &BytesPolicy,
        )
        .expect("mixed checkout plan");
    let planned_steps = plan.steps().len();
    let mut transaction = CountingTransaction::default();
    let receipt = graph
        .execute_navigation(plan, &mut transaction)
        .expect("atomic checkout");
    assert_eq!(transaction.calls, 1);
    assert_eq!(transaction.steps, planned_steps);

    let projection_started = Instant::now();
    let request = ForkProjectionPageRequest::new(0, PAGE_SIZE).expect("bounded page");
    let summary = graph.project_summary().expect("summary projection");
    let path = graph
        .project_default_path_page(request)
        .expect("default path projection");
    let branches = graph
        .project_branch_page(request)
        .expect("branch projection");
    let projection_micros = projection_started.elapsed().as_micros();
    assert!(path.entries().len() <= PAGE_SIZE);
    assert!(branches.branches().len() <= PAGE_SIZE);

    let epoch = HistoryAuthorityEpoch::new(AUTHORITY_EPOCH).expect("authority epoch");
    let snapshot = ForkSnapshot::from_summary(epoch, &summary).expect("snapshot projection");
    let path_snapshot = ForkPathPageSnapshot::from_page(epoch, &path).expect("path protocol");
    let branch_snapshot =
        longhorn_history_tree::ForkBranchPageSnapshot::from_page(epoch, &branches)
            .expect("branch protocol");
    // Card 183. The continuations at the path's newest entry, so the packed
    // artifact proves the node-centric surface as well as the flat one.
    let continuation_anchor = path.entries().first().map(|entry| entry.entry_id().clone());
    let continuations = graph
        .project_continuations(continuation_anchor.as_ref(), request)
        .expect("continuation projection");
    assert!(continuations.continuations().len() <= PAGE_SIZE);
    let continuation_snapshot =
        longhorn_history_tree::ForkContinuationPageSnapshot::from_page(epoch, &continuations)
            .expect("continuation protocol");
    let receipt_projection = ForkNavigationReceiptProjection::from_receipt(&receipt);
    let navigation_result = ForkNavigationResult::Committed {
        snapshot: snapshot.clone(),
        receipt: receipt_projection,
    };
    let changed_event = ForkChangedEvent {
        protocol_version: longhorn_history_tree::ForkHistoryProtocolVersion::CURRENT,
        authority_epoch: epoch,
        history_id: graph.history_id().clone(),
        previous_revision: Some(receipt.previous_revision()),
        committed_revision: receipt.committed_revision(),
        kind: ForkChangedKind::Navigation,
    };

    let mut checkpoint_graph = graph.clone();
    let current = checkpoint_graph.current_node_id().cloned();
    checkpoint_graph
        .register_checkpoint(
            checkpoint_graph.revision(),
            ForkCheckpointId::new("checkpoint:artifact").expect("checkpoint id"),
            current.clone(),
            "consumer://artifact-checkpoint".to_owned(),
        )
        .expect("checkpoint registration");
    let replay = checkpoint_graph
        .replay_cost(current.as_ref())
        .expect("checkpoint replay cost");
    assert_eq!(replay.entry_count(), 0);

    let mut pruning_graph = graph.clone();
    let pruning_before = pruning_graph.clone();
    let pruning_rejected = pruning_graph
        .prune_to(
            pruning_graph.revision(),
            ForkRetentionLimits::new(
                pruning_graph.retained_entry_count() - 1,
                pruning_graph.retained_encoded_weight(),
            )
            .expect("retention limits"),
        )
        .is_err();
    assert!(pruning_rejected);
    assert_eq!(pruning_graph, pruning_before);

    let mut stale_graph = graph.clone();
    let stale_before = stale_graph.clone();
    let stale_rejected = stale_graph
        .record_applied(ForkRecord::new(
            HistoryRevision::INITIAL,
            entry_id("entry:stale"),
            metadata("Stale mutation"),
            payload_bytes as u64,
            vec![0; payload_bytes],
            None,
        ))
        .is_err();
    assert!(stale_rejected);
    assert_eq!(stale_graph, stale_before);
    let truncated_rejected = persistence
        .load(base.history_id(), &encoded[..encoded.len() / 2])
        .is_err();
    assert!(truncated_rejected);

    let public_trace = json!({
        "historyId": summary.history_id().as_str(),
        "revision": summary.revision().get(),
        "currentBranchId": summary.current_branch_id().as_str(),
        "currentEntryId": summary.current_entry_id().map(HistoryEntryId::as_str),
        "undoDepth": summary.undo_depth(),
        "redoDepth": summary.redo_depth(),
        "retainedEntryCount": summary.retained_entry_count(),
        "branchCount": summary.branch_count(),
        "alternatePathCount": summary.alternate_path_count(),
        "pathEntryIds": path.entries().iter().map(|entry| entry.entry_id().as_str()).collect::<Vec<_>>(),
        "branchIds": branches.branches().iter().map(|branch| branch.branch_id().as_str()).collect::<Vec<_>>(),
        "continuationAnchorId": continuation_anchor.as_ref().map(HistoryEntryId::as_str),
        "continuationEntryIds": continuations.continuations().iter().map(|continuation| continuation.entry_id().as_str()).collect::<Vec<_>>(),
        "movedEntryCount": receipt.moved_entry_ids().len(),
        "firstMovedEntryId": receipt.moved_entry_ids().first().map(HistoryEntryId::as_str),
        "lastMovedEntryId": receipt.moved_entry_ids().last().map(HistoryEntryId::as_str),
    });
    let fixture = json!({
        "shape": shape,
        "publicTrace": public_trace,
        "rendererFixture": {
            "snapshot": snapshot,
            "path": path_snapshot,
            "branches": branch_snapshot,
            "continuations": continuation_snapshot,
            "navigationResult": navigation_result,
            "changedEvent": changed_event,
        },
        "metrics": {
            "targetDepth": main_depth,
            "alternatePaths": alternates,
            "retainedNodes": base.retained_entry_count(),
            "branchRefs": base.branches().count(),
            "payloadBytesPerNode": payload_bytes,
            "retainedPayloadBytes": base.retained_encoded_weight(),
            "lcaCheckoutSteps": planned_steps,
            "denseEnvelopeBytes": encoded.len(),
            "numericArrayBaselineBytes": numeric_baseline,
            "encodeMicros": encode_micros,
            "loadMicros": load_micros,
            "boundedProjectionMicros": projection_micros,
            "pathRecordsReturned": path.entries().len(),
            "branchRecordsReturned": branches.branches().len(),
        },
        "failures": {
            "staleRecordRejectedWithoutMutation": stale_rejected,
            "truncatedEnvelopeRejected": truncated_rejected,
            "protectedPruneRejectedWithoutMutation": pruning_rejected,
            "checkpointReplayEntries": replay.entry_count(),
            "oversizedProjectionRejected": ForkProjectionPageRequest::new(0, 257).is_err(),
        },
        "hostEvent": host_event(),
    });
    let serialized = serde_json::to_string(&fixture).expect("proof JSON");
    assert!(!serialized.to_lowercase().contains("\"payload\""));
    println!("{serialized}");
}

fn shaped_graph(
    shape: &str,
    main_depth: usize,
    alternates: usize,
    payload_bytes: usize,
) -> ForkHistory<Vec<u8>> {
    let mut nodes = Vec::with_capacity(main_depth + alternates);
    let mut parent = None;
    for index in 1..=main_depth {
        let entry = entry_id(&format!("entry:main-{index:04}"));
        nodes.push(dense_node(entry.clone(), parent, index, payload_bytes));
        parent = Some(entry);
    }
    let main_head = parent.expect("main head");
    let anchor = entry_id(&format!("entry:main-{:04}", main_depth / 2));
    let mut branches = vec![ForkBranch::new(
        branch_id("branch:main"),
        Some(main_head.clone()),
        branch_metadata("Main", true),
    )];
    for index in 1..=alternates {
        let entry = entry_id(&format!("entry:alternate-{index:02}"));
        nodes.push(dense_node(
            entry.clone(),
            Some(anchor.clone()),
            main_depth + index,
            payload_bytes,
        ));
        branches.push(ForkBranch::new(
            branch_id(&format!("branch:alternate-{index:02}")),
            Some(entry),
            branch_metadata(&format!("Alternate {index}"), false),
        ));
    }
    ForkHistory::from_state(
        ForkHistoryState::new(
            history_id(&format!("history:{shape}-artifact")),
            HistoryRevision::new((main_depth + alternates) as u64),
            branch_id("branch:main"),
            Some(main_head),
            HistoryEntrySequence::new((main_depth + alternates + 1) as u64).expect("next sequence"),
        )
        .with_nodes(nodes)
        .with_branches(branches)
        .with_preferred_children(vec![ForkPreferredChild::new(
            None,
            entry_id("entry:main-0001"),
        )]),
    )
    .expect("valid shaped graph")
}

fn dense_node(
    entry_id: HistoryEntryId,
    parent_entry_id: Option<HistoryEntryId>,
    sequence: usize,
    payload_bytes: usize,
) -> ForkHistoryNode<Vec<u8>> {
    ForkHistoryNode::new(
        entry_id,
        parent_entry_id,
        metadata("Pulse mutation"),
        HistoryEntrySequence::new(sequence as u64).expect("entry sequence"),
        HistoryRevision::new(sequence as u64),
        payload_bytes as u64,
        vec![(sequence % 251) as u8; payload_bytes],
    )
}

fn metadata(label: &str) -> HistoryEntryMetadata {
    HistoryEntryMetadata::new(
        HistoryLabel::new(label).expect("history label"),
        Some(HistoryKindId::new("fixture:mutation").expect("kind id")),
        Some(HistoryGroupId::new("fixture:group").expect("group id")),
    )
}

fn branch_metadata(name: &str, pinned: bool) -> ForkBranchMetadata {
    ForkBranchMetadata::new(
        Some(name.to_owned()),
        Some(format!("{name} fixture branch")),
        pinned,
    )
    .expect("branch metadata")
}

fn history_id(value: &str) -> HistoryId {
    HistoryId::new(value).expect("history id")
}

fn entry_id(value: &str) -> HistoryEntryId {
    HistoryEntryId::new(value).expect("entry id")
}

fn branch_id(value: &str) -> ForkBranchId {
    ForkBranchId::new(value).expect("branch id")
}

fn plan_id(value: &str) -> HistoryPlanId {
    HistoryPlanId::new(value).expect("plan id")
}

#[cfg(feature = "tauri-host")]
fn host_event() -> Option<&'static str> {
    Some(longhorn_tauri_history_tree::FORK_HISTORY_CHANGED_EVENT)
}

#[cfg(not(feature = "tauri-host"))]
fn host_event() -> Option<&'static str> {
    None
}
