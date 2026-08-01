//! Repeatable Card 068 topology, persistence, projection, and pruning evidence.

use std::{convert::Infallible, mem::size_of, time::Instant};

use longhorn_core::{HistoryEntryId, HistoryId, HistoryKindId, HistoryPlanId};
use longhorn_history::{
    HistoryCoalesce, HistoryCoalesceContext, HistoryEntryMetadata, HistoryLabel,
    HistoryNavigationStep, HistoryNavigationTransactionFailure, HistoryPayloadCodecFamily,
    HistoryPayloadCodecVersion, HistoryPolicy,
};
use longhorn_history_tree_prototype::{
    ForkBranchId, ForkBranchMetadata, ForkBranchSeed, ForkCheckpointId, ForkHistory,
    ForkNavigationPlan, ForkNavigationTarget, ForkNavigationTransaction, ForkPayloadCodec,
    ForkPersistence, ForkPruningOutcome, ForkRecord, ForkRetentionLimits,
};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq)]
struct BenchPayload {
    delta: i64,
    bytes: Vec<u8>,
}

struct BenchPolicy;

impl HistoryPolicy<BenchPayload> for BenchPolicy {
    type Error = Infallible;

    fn inverse(&self, payload: &BenchPayload) -> Result<BenchPayload, Self::Error> {
        Ok(BenchPayload {
            delta: -payload.delta,
            bytes: payload.bytes.clone(),
        })
    }

    fn is_noop(&self, payload: &BenchPayload) -> bool {
        payload.delta == 0
    }

    fn encoded_weight(&self, payload: &BenchPayload) -> Result<u64, Self::Error> {
        Ok(u64::try_from(payload.bytes.len()).expect("fixture size") + 8)
    }

    fn coalesce(
        &self,
        _previous: &BenchPayload,
        _incoming: &BenchPayload,
        _context: HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<BenchPayload>, Self::Error> {
        Ok(HistoryCoalesce::KeepSeparate)
    }
}

struct ModelTransaction<'model> {
    model: &'model mut i64,
}

impl ForkNavigationTransaction<BenchPayload> for ModelTransaction<'_> {
    type Error = Infallible;

    fn apply(
        &mut self,
        plan: &ForkNavigationPlan<BenchPayload>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        for step in plan.steps() {
            match step {
                HistoryNavigationStep::Undo { payload, .. }
                | HistoryNavigationStep::Redo { payload, .. } => {
                    *self.model += payload.delta;
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
struct BenchCodec {
    family: HistoryPayloadCodecFamily,
}

impl BenchCodec {
    fn new() -> Self {
        Self {
            family: HistoryPayloadCodecFamily::new("fixture.benchmark").expect("codec family"),
        }
    }
}

impl ForkPayloadCodec<BenchPayload> for BenchCodec {
    type Error = &'static str;

    fn family(&self) -> &HistoryPayloadCodecFamily {
        &self.family
    }

    fn version(&self) -> HistoryPayloadCodecVersion {
        HistoryPayloadCodecVersion::new(1)
    }

    fn encode(&self, payload: &BenchPayload) -> Result<Vec<u8>, Self::Error> {
        let mut bytes = Vec::with_capacity(payload.bytes.len() + 8);
        bytes.extend_from_slice(&payload.delta.to_le_bytes());
        bytes.extend_from_slice(&payload.bytes);
        Ok(bytes)
    }

    fn decode(&self, bytes: &[u8]) -> Result<BenchPayload, Self::Error> {
        let (delta, payload) = bytes.split_at_checked(8).ok_or("short payload")?;
        Ok(BenchPayload {
            delta: i64::from_le_bytes(delta.try_into().map_err(|_| "invalid delta")?),
            bytes: payload.to_vec(),
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Measurement {
    shape: &'static str,
    target_depth: usize,
    target_alternate_paths: usize,
    payload_bytes_per_entry: usize,
    retained_entries: usize,
    retained_branches: usize,
    retained_payload_bytes: u64,
    payload_allocation_count: usize,
    authority_root_bytes: usize,
    construction_micros: u64,
    checkout_steps: usize,
    checkout_plan_micros: u64,
    checkout_apply_micros: u64,
    checkpoint_replay_entries: usize,
    checkpoint_replay_payload_bytes: u64,
    projection_entries: usize,
    projection_micros: u64,
    encoded_envelope_bytes: usize,
    encode_micros: u64,
    decode_micros: u64,
    pruned_entries: usize,
    prune_micros: u64,
}

struct Fixture {
    history: ForkHistory<BenchPayload>,
    model: i64,
    main_branch: ForkBranchId,
    main_head: HistoryEntryId,
    anchor: HistoryEntryId,
    last_alternate: HistoryEntryId,
}

fn main() {
    let results = [
        measure("document", 128, 4, 24),
        measure("loophole-shaped", 2_048, 64, 248),
    ];
    println!(
        "{}",
        serde_json::to_string_pretty(&results).expect("measurement JSON")
    );
}

fn measure(
    shape: &'static str,
    depth: usize,
    alternate_paths: usize,
    body_bytes: usize,
) -> Measurement {
    let construction_started = Instant::now();
    let mut fixture = build_fixture(shape, depth, alternate_paths, body_bytes);
    let construction_micros = micros(construction_started.elapsed());

    let plan_started = Instant::now();
    let checkout = fixture
        .history
        .plan_navigation(
            plan_id("plan:benchmark-checkout"),
            fixture.history.revision(),
            ForkNavigationTarget::Checkout {
                branch_id: fixture.main_branch.clone(),
                entry_id: fixture.main_head.clone(),
            },
            &BenchPolicy,
        )
        .expect("checkout plan");
    let checkout_plan_micros = micros(plan_started.elapsed());
    let checkout_steps = checkout.steps().len();
    let apply_started = Instant::now();
    fixture
        .history
        .execute_navigation(
            checkout,
            &mut ModelTransaction {
                model: &mut fixture.model,
            },
        )
        .expect("checkout apply");
    let checkout_apply_micros = micros(apply_started.elapsed());

    fixture
        .history
        .register_checkpoint(
            fixture.history.revision(),
            ForkCheckpointId::new("checkpoint:anchor").expect("checkpoint id"),
            Some(fixture.anchor.clone()),
            format!("consumer://{shape}/anchor"),
        )
        .expect("checkpoint");
    let replay = fixture
        .history
        .replay_cost(Some(&fixture.last_alternate))
        .expect("replay cost");

    let projection_started = Instant::now();
    let projection = fixture
        .history
        .alternate_projection()
        .expect("alternate projection");
    let projection_micros = micros(projection_started.elapsed());
    let projection_entries = projection
        .derived_paths()
        .iter()
        .map(|path| path.entry_ids().len())
        .sum();

    let persistence = ForkPersistence::without_structural_migration(BenchCodec::new());
    let encode_started = Instant::now();
    let encoded = persistence.encode(&fixture.history).expect("encode graph");
    let encode_micros = micros(encode_started.elapsed());
    let decode_started = Instant::now();
    let loaded = persistence
        .load(fixture.history.history_id(), &encoded)
        .expect("decode graph");
    let decode_micros = micros(decode_started.elapsed());
    assert_eq!(loaded.history(), &fixture.history);

    let retained_entries = fixture.history.retained_entry_count();
    let retained_branches = fixture.history.branches().count();
    let retained_payload_bytes = fixture.history.retained_encoded_weight();
    let mut prunable = fixture.history.clone();
    let prune_started = Instant::now();
    let pruning = prunable
        .prune_to(
            prunable.revision(),
            ForkRetentionLimits::new(
                depth,
                u64::try_from(depth).expect("fixture depth")
                    * u64::try_from(body_bytes + 8).expect("fixture payload"),
            )
            .expect("retention limits"),
        )
        .expect("prune");
    let prune_micros = micros(prune_started.elapsed());
    let ForkPruningOutcome::Pruned(receipt) = pruning else {
        panic!("alternate paths must prune");
    };
    assert_eq!(receipt.pruned_nodes().len(), alternate_paths);

    Measurement {
        shape,
        target_depth: depth,
        target_alternate_paths: alternate_paths,
        payload_bytes_per_entry: body_bytes + 8,
        retained_entries,
        retained_branches,
        retained_payload_bytes,
        payload_allocation_count: retained_entries,
        authority_root_bytes: size_of::<ForkHistory<BenchPayload>>(),
        construction_micros,
        checkout_steps,
        checkout_plan_micros,
        checkout_apply_micros,
        checkpoint_replay_entries: replay.entry_count(),
        checkpoint_replay_payload_bytes: replay.encoded_weight(),
        projection_entries,
        projection_micros,
        encoded_envelope_bytes: encoded.len(),
        encode_micros,
        decode_micros,
        pruned_entries: receipt.pruned_nodes().len(),
        prune_micros,
    }
}

fn build_fixture(shape: &str, depth: usize, alternate_paths: usize, body_bytes: usize) -> Fixture {
    let main_branch = branch_id("branch:main");
    let mut history = ForkHistory::new(
        HistoryId::new(format!("history:{shape}")).expect("history id"),
        main_branch.clone(),
        ForkBranchMetadata::new(Some("Main".to_owned()), None, true).expect("main metadata"),
    );
    let mut model = 0_i64;
    let anchor_index = depth / 2;
    for index in 1..=depth {
        let entry_id = entry_id(index);
        model += 1;
        history
            .record_applied(ForkRecord::new(
                history.revision(),
                entry_id,
                metadata(&format!("Edit {index}")),
                u64::try_from(body_bytes + 8).expect("fixture weight"),
                BenchPayload {
                    delta: 1,
                    bytes: vec![u8::try_from(index % 251).expect("fixture byte"); body_bytes],
                },
                None,
            ))
            .expect("linear record");
    }
    let main_head = entry_id(depth);
    let anchor = entry_id(anchor_index);
    let mut last_alternate = anchor.clone();
    for branch_index in 0..alternate_paths {
        let checkout = history
            .plan_navigation(
                plan_id(&format!("plan:anchor-{branch_index}")),
                history.revision(),
                ForkNavigationTarget::Checkout {
                    branch_id: main_branch.clone(),
                    entry_id: anchor.clone(),
                },
                &BenchPolicy,
            )
            .expect("anchor plan");
        history
            .execute_navigation(checkout, &mut ModelTransaction { model: &mut model })
            .expect("anchor checkout");
        last_alternate = HistoryEntryId::new(format!("entry:alternate-{branch_index:03}"))
            .expect("alternate id");
        model += 1;
        history
            .record_applied(ForkRecord::new(
                history.revision(),
                last_alternate.clone(),
                metadata(&format!("Alternate {branch_index}")),
                u64::try_from(body_bytes + 8).expect("fixture weight"),
                BenchPayload {
                    delta: 1,
                    bytes: vec![
                        u8::try_from(branch_index % 251).expect("fixture byte");
                        body_bytes
                    ],
                },
                Some(ForkBranchSeed::new(
                    branch_id(&format!("branch:alternate-{branch_index:03}")),
                    ForkBranchMetadata::new(None, None, false).expect("alternate metadata"),
                )),
            ))
            .expect("alternate record");
    }
    Fixture {
        history,
        model,
        main_branch,
        main_head,
        anchor,
        last_alternate,
    }
}

fn entry_id(index: usize) -> HistoryEntryId {
    HistoryEntryId::new(format!("entry:{index:06}")).expect("entry id")
}

fn branch_id(value: &str) -> ForkBranchId {
    ForkBranchId::new(value).expect("branch id")
}

fn plan_id(value: &str) -> HistoryPlanId {
    HistoryPlanId::new(value).expect("plan id")
}

fn metadata(label: &str) -> HistoryEntryMetadata {
    HistoryEntryMetadata::new(
        HistoryLabel::new(label).expect("label"),
        Some(HistoryKindId::new("fixture:mutation").expect("kind")),
        None,
    )
}

fn micros(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}
