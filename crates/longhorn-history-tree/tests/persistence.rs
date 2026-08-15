//! Dense graph persistence, migration, and hostile-input evidence.

use std::{convert::Infallible, error::Error, fmt, time::Instant};

use longhorn_core::{
    CompatibilityStore, FutureSchemaRefused, HistoryEntryId, HistoryGroupId, HistoryId,
    HistoryKindId, HistoryRevision,
};
use longhorn_history::{
    HistoryEntryMetadata, HistoryEntrySequence, HistoryLabel, HistoryPayloadCodec,
    HistoryPayloadCodecFamily, HistoryPayloadCodecVersion, HistoryPayloadMigrationStep,
    HistoryPayloadMigrationTarget, HistoryRecordedAt,
};
use longhorn_history_tree::{
    ForkBranch, ForkBranchId, ForkBranchMetadata, ForkBranchSeed, ForkCheckpointId, ForkHistory,
    ForkHistoryNode, ForkHistoryState, ForkHistoryStateError, ForkLoadError, ForkLoadOutcome,
    ForkPersistence, ForkPersistenceLimits, ForkPreferredChild, ForkRecord,
    ForkStructuralMigration, ForkStructuralMigrationStep, ForkStructuralMigrationTarget,
};
use proptest::prelude::*;
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Mutation {
    delta: i64,
    body: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CodecError {
    Short,
    Checksum,
}

type FixtureLoadError = ForkLoadError<CodecError, Infallible>;
type FixtureErrorMatcher = fn(&FixtureLoadError) -> bool;

impl fmt::Display for CodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("fixture codec failed")
    }
}

impl Error for CodecError {}

#[derive(Clone)]
struct MutationCodec {
    family: HistoryPayloadCodecFamily,
    version: HistoryPayloadCodecVersion,
}

impl MutationCodec {
    fn version_one() -> Self {
        Self::new("fixture.fork-mutation", 1)
    }

    fn version_two() -> Self {
        Self::new("fixture.fork-mutation", 2)
    }

    fn new(family: &str, version: u32) -> Self {
        Self {
            family: HistoryPayloadCodecFamily::new(family).expect("fixture codec family"),
            version: HistoryPayloadCodecVersion::new(version),
        }
    }
}

impl HistoryPayloadCodec<Mutation> for MutationCodec {
    type Error = CodecError;

    fn family(&self) -> &HistoryPayloadCodecFamily {
        &self.family
    }

    fn version(&self) -> HistoryPayloadCodecVersion {
        self.version
    }

    fn encode(&self, payload: &Mutation) -> Result<Vec<u8>, Self::Error> {
        let mut bytes = Vec::with_capacity(payload.body.len() + 9);
        bytes.extend_from_slice(&payload.delta.to_le_bytes());
        bytes.extend_from_slice(&payload.body);
        if self.version.get() == 2 {
            bytes.push(checksum(&bytes));
        }
        Ok(bytes)
    }

    fn decode(&self, bytes: &[u8]) -> Result<Mutation, Self::Error> {
        let minimum = if self.version.get() == 1 { 8 } else { 9 };
        if bytes.len() < minimum {
            return Err(CodecError::Short);
        }
        let payload_end = if self.version.get() == 1 {
            bytes.len()
        } else {
            let (checksum_byte, payload) = bytes.split_last().ok_or(CodecError::Short)?;
            if *checksum_byte != checksum(payload) {
                return Err(CodecError::Checksum);
            }
            payload.len()
        };
        let delta = i64::from_le_bytes(bytes[..8].try_into().map_err(|_| CodecError::Short)?);
        Ok(Mutation {
            delta,
            body: bytes[8..payload_end].to_vec(),
        })
    }

    fn migrate_one(
        &self,
        from: HistoryPayloadCodecVersion,
        mut bytes: Vec<u8>,
        target: HistoryPayloadMigrationTarget<'_>,
    ) -> Result<Option<HistoryPayloadMigrationStep>, Self::Error> {
        if from.get() == 1 && target.version().get() == 2 {
            bytes.push(checksum(&bytes));
            return Ok(Some(HistoryPayloadMigrationStep::new(
                HistoryPayloadCodecVersion::new(2),
                bytes,
            )));
        }
        Ok(None)
    }
}

fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0, |sum, byte| sum.wrapping_add(*byte))
}

fn history_id(value: &str) -> HistoryId {
    HistoryId::new(value).expect("fixture history id")
}

fn entry_id(value: &str) -> HistoryEntryId {
    HistoryEntryId::new(value).expect("fixture entry id")
}

fn branch_id(value: &str) -> ForkBranchId {
    ForkBranchId::new(value).expect("fixture branch id")
}

fn metadata(label: &str) -> HistoryEntryMetadata {
    HistoryEntryMetadata::new(
        HistoryLabel::new(label).expect("fixture label"),
        Some(HistoryKindId::new("fixture:mutation").expect("fixture kind")),
        Some(HistoryGroupId::new("fixture:group").expect("fixture group")),
    )
}

fn branch_metadata(name: &str, pinned: bool) -> ForkBranchMetadata {
    ForkBranchMetadata::new(
        Some(name.to_owned()),
        Some(format!("{name} fixture branch")),
        pinned,
    )
    .expect("fixture branch metadata")
}

fn limits() -> ForkPersistenceLimits {
    ForkPersistenceLimits::new(64 * 1_024 * 1_024).expect("fixture persistence limits")
}

fn mutation(delta: i64, body: &[u8]) -> Mutation {
    Mutation {
        delta,
        body: body.to_vec(),
    }
}

fn mutation_weight(codec_version: u32, body: &[u8]) -> u64 {
    u64::try_from(8 + body.len() + usize::from(codec_version == 2)).expect("fixture weight")
}

fn record(
    graph: &mut ForkHistory<Mutation>,
    id: &str,
    payload: Mutation,
    codec_version: u32,
    divergent_branch: Option<ForkBranchSeed>,
) {
    let encoded_weight = mutation_weight(codec_version, &payload.body);
    graph
        .record_applied(ForkRecord::new(
            graph.revision(),
            entry_id(id),
            metadata(id),
            encoded_weight,
            payload,
            divergent_branch,
        ))
        .expect("fixture record");
}

fn graph(codec_version: u32) -> ForkHistory<Mutation> {
    let mut graph = ForkHistory::new(
        history_id("history:fork-fixture"),
        branch_id("branch:main"),
        branch_metadata("Main", true),
    );
    record(
        &mut graph,
        "entry:a",
        mutation(1, b"alpha"),
        codec_version,
        None,
    );
    record(
        &mut graph,
        "entry:b",
        mutation(2, b"bravo"),
        codec_version,
        None,
    );

    let state = graph.into_state();
    let mut graph = ForkHistory::from_state(
        ForkHistoryState::new(
            state.history_id().clone(),
            state.revision(),
            branch_id("branch:main"),
            Some(entry_id("entry:a")),
            state.next_sequence(),
        )
        .with_nodes(state.nodes().to_vec())
        .with_branches(state.branches().to_vec())
        .with_preferred_children(state.preferred_children().to_vec()),
    )
    .expect("fixture position behind branch head");
    record(
        &mut graph,
        "entry:c",
        mutation(-2, b"charlie"),
        codec_version,
        Some(ForkBranchSeed::new(
            branch_id("branch:alternate"),
            branch_metadata("Alternate", false),
        )),
    );
    graph
        .register_checkpoint(
            graph.revision(),
            ForkCheckpointId::new("checkpoint:a").expect("fixture checkpoint id"),
            Some(entry_id("entry:a")),
            "consumer://fixture/checkpoint-a".to_owned(),
        )
        .expect("fixture checkpoint");
    graph
}

fn persistence_v1()
-> ForkPersistence<MutationCodec, longhorn_history_tree::NoForkStructuralMigration> {
    ForkPersistence::without_structural_migration(MutationCodec::version_one(), limits())
}

fn encoded_v1() -> (ForkHistory<Mutation>, Vec<u8>) {
    let graph = graph(1);
    let encoded = persistence_v1().encode(&graph).expect("fixture encode");
    (graph, encoded)
}

#[test]
fn golden_dense_envelope_round_trips_exact_graph_deterministically() {
    let (graph, encoded) = encoded_v1();
    let document: Value = serde_json::from_slice(&encoded).expect("fixture JSON");
    assert!(
        document["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .all(|node| node["payload"].is_string())
    );
    assert!(!String::from_utf8_lossy(&encoded).contains("\"payload\": ["));

    let loaded = persistence_v1()
        .load(graph.history_id(), &encoded)
        .expect("fixture load");
    assert_eq!(loaded.receipt().outcome(), ForkLoadOutcome::Preserved);
    assert_eq!(loaded.history(), &graph);
    assert_eq!(persistence_v1().encode(loaded.history()).unwrap(), encoded);
    assert_eq!(persistence_v1().encode(&graph).unwrap(), encoded);

    // The checked fixture freezes field names, ordering, and base64 bytes.
    assert_eq!(
        String::from_utf8(encoded).unwrap(),
        include_str!("../fixtures/history/tree-v1.json").trim_end()
    );
}

#[derive(Clone, Copy)]
struct VersionZeroStructuralMigration;

impl ForkStructuralMigration for VersionZeroStructuralMigration {
    type Error = Infallible;

    fn migrate_one(
        &self,
        from: u32,
        mut document: Value,
        target: ForkStructuralMigrationTarget,
    ) -> Result<Option<ForkStructuralMigrationStep>, Self::Error> {
        if from == 0 && target.version() == 1 {
            document["structuralVersion"] = Value::from(1);
            document["checkpoints"] = Value::Array(Vec::new());
            return Ok(Some(ForkStructuralMigrationStep::new(1, document)));
        }
        Ok(None)
    }
}

fn as_structural_zero(mut document: Value) -> Value {
    document["structuralVersion"] = Value::from(0);
    document
        .as_object_mut()
        .expect("envelope object")
        .remove("checkpoints");
    document
}

#[test]
fn structural_and_payload_versions_migrate_independently() {
    let current = ForkPersistence::new(
        MutationCodec::version_two(),
        VersionZeroStructuralMigration,
        limits(),
    );
    let source_v1 = persistence_v1().encode(&graph(1)).unwrap();
    let source_v2 =
        ForkPersistence::without_structural_migration(MutationCodec::version_two(), limits())
            .encode(&graph(2))
            .unwrap();

    let cases = [
        (
            serde_json::from_slice(&source_v2).unwrap(),
            ForkLoadOutcome::Preserved,
        ),
        (
            as_structural_zero(serde_json::from_slice(&source_v2).unwrap()),
            ForkLoadOutcome::Migrated {
                structural: true,
                payload: false,
            },
        ),
        (
            serde_json::from_slice(&source_v1).unwrap(),
            ForkLoadOutcome::Migrated {
                structural: false,
                payload: true,
            },
        ),
        (
            as_structural_zero(serde_json::from_slice(&source_v1).unwrap()),
            ForkLoadOutcome::Migrated {
                structural: true,
                payload: true,
            },
        ),
    ];

    for (document, expected) in cases {
        let result = current
            .load(
                &history_id("history:fork-fixture"),
                &serde_json::to_vec(&document).unwrap(),
            )
            .expect("compatible source");
        assert_eq!(result.receipt().outcome(), expected);
        assert!(
            result
                .history()
                .nodes()
                .all(|node| { node.encoded_weight() == mutation_weight(2, &node.payload().body) })
        );
        let current_bytes = current.encode(result.history()).unwrap();
        let reloaded = current
            .load(result.history().history_id(), &current_bytes)
            .unwrap();
        assert_eq!(current.encode(reloaded.history()).unwrap(), current_bytes);
    }
}

#[derive(Clone, Copy)]
struct SkippingStructuralMigration;

impl ForkStructuralMigration for SkippingStructuralMigration {
    type Error = Infallible;

    fn migrate_one(
        &self,
        _: u32,
        mut document: Value,
        _: ForkStructuralMigrationTarget,
    ) -> Result<Option<ForkStructuralMigrationStep>, Self::Error> {
        document["structuralVersion"] = Value::from(2);
        Ok(Some(ForkStructuralMigrationStep::new(2, document)))
    }
}

#[test]
fn migrations_require_registered_exact_next_steps() {
    let (_, encoded) = encoded_v1();
    let structural_zero = serde_json::to_vec(&as_structural_zero(
        serde_json::from_slice(&encoded).unwrap(),
    ))
    .unwrap();
    assert!(matches!(
        persistence_v1().load(&history_id("history:fork-fixture"), &structural_zero),
        Err(ForkLoadError::MissingStructuralMigration { from: 0 })
    ));

    let skipping = ForkPersistence::new(
        MutationCodec::version_one(),
        SkippingStructuralMigration,
        limits(),
    );
    assert!(matches!(
        skipping.load(&history_id("history:fork-fixture"), &structural_zero),
        Err(ForkLoadError::InvalidStructuralMigration {
            from: 0,
            produced: 2
        })
    ));

    let version_three = ForkPersistence::without_structural_migration(
        MutationCodec::new("fixture.fork-mutation", 3),
        limits(),
    );
    assert!(matches!(
        version_three.load(&history_id("history:fork-fixture"), &encoded),
        Err(ForkLoadError::MissingPayloadMigration { from, .. }) if from.get() == 1
    ));
}

#[test]
fn foreign_future_corrupt_truncated_and_oversized_sources_reject() {
    let (live, encoded) = encoded_v1();
    let before = live.clone();
    let persistence = persistence_v1();

    let cases: Vec<(Value, FixtureErrorMatcher)> = vec![
        (
            mutated(&encoded, |value| {
                value["formatFamily"] = Value::from("foreign.tree")
            }),
            |error| matches!(error, ForkLoadError::ForeignFormatFamily { .. }),
        ),
        (
            mutated(&encoded, |value| {
                value["structuralVersion"] = Value::from(2)
            }),
            |error| matches!(error, ForkLoadError::FutureStructuralVersion { .. }),
        ),
        (
            mutated(&encoded, |value| {
                value["historyId"] = Value::from("history:other")
            }),
            |error| matches!(error, ForkLoadError::ForeignHistory { .. }),
        ),
        (
            mutated(&encoded, |value| {
                value["payloadCodec"]["family"] = Value::from("fixture.other")
            }),
            |error| matches!(error, ForkLoadError::ForeignPayloadCodecFamily { .. }),
        ),
        (
            mutated(&encoded, |value| {
                value["payloadCodec"]["version"] = Value::from(2)
            }),
            |error| matches!(error, ForkLoadError::FuturePayloadCodecVersion { .. }),
        ),
        (
            mutated(&encoded, |value| value["unexpected"] = Value::Bool(true)),
            |error| matches!(error, ForkLoadError::InvalidEnvelope(_)),
        ),
        (
            mutated(&encoded, |value| {
                value["nodes"][0]["payload"] = Value::from("***")
            }),
            |error| matches!(error, ForkLoadError::InvalidEnvelope(_)),
        ),
        (
            mutated(&encoded, |value| {
                value["nodes"][0]["encodedWeight"] = Value::from(999)
            }),
            |error| matches!(error, ForkLoadError::PayloadWeightMismatch { .. }),
        ),
    ];
    for (document, expected) in cases {
        let error = persistence
            .load(live.history_id(), &serde_json::to_vec(&document).unwrap())
            .expect_err("hostile source must reject");
        assert!(expected(&error), "unexpected error: {error:?}");
        assert_eq!(live, before);
    }

    assert!(matches!(
        persistence.load(live.history_id(), &encoded[..encoded.len() / 2]),
        Err(ForkLoadError::InvalidJson(_))
    ));
    assert!(matches!(
        persistence.load(live.history_id(), b"not JSON"),
        Err(ForkLoadError::InvalidJson(_))
    ));
    let bounded = ForkPersistence::without_structural_migration(
        MutationCodec::version_one(),
        ForkPersistenceLimits::new(128).unwrap(),
    );
    assert!(matches!(
        bounded.load(live.history_id(), &encoded),
        Err(ForkLoadError::EnvelopeTooLarge { .. })
    ));
    assert!(matches!(
        bounded.encode(&live),
        Err(longhorn_history_tree::ForkEncodeError::EnvelopeTooLarge { .. })
    ));
    let version_two =
        ForkPersistence::without_structural_migration(MutationCodec::version_two(), limits());
    assert!(matches!(
        version_two.encode(&live),
        Err(longhorn_history_tree::ForkEncodeError::PayloadWeightMismatch { .. })
    ));
    assert!(ForkPersistenceLimits::new(0).is_err());
    assert!(
        ForkPersistenceLimits::new(longhorn_history_tree::MAXIMUM_FORK_HISTORY_ENVELOPE_BYTES + 1)
            .is_err()
    );
}

fn mutated(bytes: &[u8], mutate: impl FnOnce(&mut Value)) -> Value {
    let mut value = serde_json::from_slice(bytes).expect("fixture JSON");
    mutate(&mut value);
    value
}

#[test]
fn complete_topology_corruption_matrix_rejects_before_authority_returns() {
    let (live, encoded) = encoded_v1();
    let cases = [
        (
            mutated(&encoded, |value| {
                value["nodes"][1]["parentEntryId"] = Value::from("entry:missing");
            }),
            ForkHistoryStateError::InvalidParent(entry_id("entry:b")),
        ),
        (
            mutated(&encoded, |value| {
                value["branches"][0]["headEntryId"] = Value::from("entry:missing");
            }),
            ForkHistoryStateError::InvalidBranchHead(branch_id("branch:alternate")),
        ),
        (
            mutated(&encoded, |value| {
                value["currentNodeId"] = Value::from("entry:missing");
            }),
            ForkHistoryStateError::InvalidCurrentNode,
        ),
        (
            mutated(&encoded, |value| {
                value["preferredChildren"][0]["childEntryId"] = Value::from("entry:b");
            }),
            ForkHistoryStateError::InvalidPreferredChild(entry_id("entry:b")),
        ),
        (
            mutated(&encoded, |value| {
                value["nodes"][1]["sequence"] = value["nodes"][0]["sequence"].clone();
            }),
            ForkHistoryStateError::DuplicateSequence(1),
        ),
        (
            mutated(&encoded, |value| {
                value["nodes"][1]["committedRevision"] =
                    value["nodes"][0]["committedRevision"].clone();
            }),
            ForkHistoryStateError::DuplicateCommittedRevision(1),
        ),
        (
            mutated(&encoded, |value| value["nextSequence"] = Value::from(1)),
            ForkHistoryStateError::InvalidNextSequence,
        ),
        (
            mutated(&encoded, |value| {
                value["checkpoints"][0]["afterEntryId"] = Value::from("entry:missing");
            }),
            ForkHistoryStateError::InvalidCheckpoint(
                ForkCheckpointId::new("checkpoint:a").unwrap(),
            ),
        ),
    ];
    for (document, expected) in cases {
        assert!(matches!(
            persistence_v1().load(
                live.history_id(),
                &serde_json::to_vec(&document).unwrap()
            ),
            Err(ForkLoadError::Validation(actual)) if actual == expected
        ));
    }
}

#[derive(Clone)]
struct DenseCodec {
    family: HistoryPayloadCodecFamily,
}

impl DenseCodec {
    fn new() -> Self {
        Self {
            family: HistoryPayloadCodecFamily::new("fixture.loophole-pulse").unwrap(),
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

#[test]
fn loophole_shaped_envelope_is_materially_denser_than_numeric_array_prototype() {
    let graph = loophole_shaped_graph();
    let persistence = ForkPersistence::without_structural_migration(DenseCodec::new(), limits());
    let encode_started = Instant::now();
    let encoded = persistence.encode(&graph).unwrap();
    let encode_micros = encode_started.elapsed().as_micros();
    let load_started = Instant::now();
    let loaded = persistence.load(graph.history_id(), &encoded).unwrap();
    let load_micros = load_started.elapsed().as_micros();
    assert_eq!(loaded.history(), &graph);

    let numeric_payload_bytes = graph
        .nodes()
        .map(|node| serde_json::to_vec(node.payload()).unwrap().len())
        .sum::<usize>();
    let base64_payload_bytes = graph
        .nodes()
        .map(|node| 4 * node.payload().len().div_ceil(3))
        .sum::<usize>();
    eprintln!(
        "loophole-shaped nodes={} payload={} base64_payload={} numeric_payload={} envelope={} encode_us={} load_us={}",
        graph.retained_entry_count(),
        graph.retained_encoded_weight(),
        base64_payload_bytes,
        numeric_payload_bytes,
        encoded.len(),
        encode_micros,
        load_micros,
    );

    assert_eq!(graph.retained_entry_count(), 2_112);
    assert_eq!(graph.retained_encoded_weight(), 540_672);
    assert_eq!(base64_payload_bytes, 726_528);
    assert!(base64_payload_bytes * 2 < numeric_payload_bytes);
    assert!(encoded.len() * 2 < 7_534_856);
}

fn loophole_shaped_graph() -> ForkHistory<Vec<u8>> {
    const MAIN_DEPTH: usize = 2_048;
    const ALTERNATES: usize = 64;
    const PAYLOAD_BYTES: usize = 256;

    let mut nodes = Vec::with_capacity(MAIN_DEPTH + ALTERNATES);
    let mut parent = None;
    for index in 1..=MAIN_DEPTH {
        let entry = entry_id(&format!("entry:main-{index:04}"));
        nodes.push(dense_node(entry.clone(), parent, index, PAYLOAD_BYTES));
        parent = Some(entry);
    }
    let main_head = parent.expect("main head");
    let anchor = entry_id("entry:main-1024");
    let mut branches = vec![ForkBranch::new(
        branch_id("branch:main"),
        Some(main_head.clone()),
        branch_metadata("Main", true),
    )];
    for index in 1..=ALTERNATES {
        let entry = entry_id(&format!("entry:alternate-{index:02}"));
        nodes.push(dense_node(
            entry.clone(),
            Some(anchor.clone()),
            MAIN_DEPTH + index,
            PAYLOAD_BYTES,
        ));
        branches.push(ForkBranch::new(
            branch_id(&format!("branch:alternate-{index:02}")),
            Some(entry),
            branch_metadata(&format!("Alternate {index}"), false),
        ));
    }

    ForkHistory::from_state(
        ForkHistoryState::new(
            history_id("history:loophole-shaped"),
            HistoryRevision::new((MAIN_DEPTH + ALTERNATES) as u64),
            branch_id("branch:main"),
            Some(main_head),
            HistoryEntrySequence::new((MAIN_DEPTH + ALTERNATES + 1) as u64).unwrap(),
        )
        .with_nodes(nodes)
        .with_branches(branches)
        .with_preferred_children(vec![
            ForkPreferredChild::new(None, entry_id("entry:main-0001")),
            // The anchor carries every alternate, so it has a real choice and
            // has to name one. Without this the graph is rejected -- and it
            // was rejected when the guard landed, which is the point: a
            // forward walk from the anchor used to stop dead and none of the
            // alternates were reachable. The fixture only measured envelope
            // density, so nothing noticed.
            ForkPreferredChild::new(Some(anchor.clone()), entry_id("entry:alternate-01")),
        ]),
    )
    .expect("Loophole-shaped graph")
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
        HistoryEntrySequence::new(sequence as u64).unwrap(),
        HistoryRevision::new(sequence as u64),
        payload_bytes as u64,
        vec![(sequence % 251) as u8; payload_bytes],
    )
}

#[test]
fn future_versions_classify_for_the_update_surface() {
    // Structural envelope and payload codec advance independently, so a
    // channel rejoin can be refused by either. Both must be recognisable to
    // the update surface without matching fork-specific error variants.
    let (live, encoded) = encoded_v1();
    let persistence = persistence_v1();
    let history = history_id("history:fork-fixture");

    let future_structural = serde_json::to_vec(&mutated(&encoded, |value| {
        value["structuralVersion"] = Value::from(2)
    }))
    .unwrap();
    let error = persistence
        .load(&history, &future_structural)
        .expect_err("a future structural version must not load");
    let refusal = error
        .future_schema_refusal()
        .expect("a future structural version must classify");
    assert_eq!(refusal.store, CompatibilityStore::HistoryTree);
    assert_eq!(refusal.found, Some(2));

    let future_payload = serde_json::to_vec(&mutated(&encoded, |value| {
        value["payloadCodec"]["version"] = Value::from(2)
    }))
    .unwrap();
    let error = persistence
        .load(&history, &future_payload)
        .expect_err("a future payload codec version must not load");
    let refusal = error
        .future_schema_refusal()
        .expect("a future payload codec version must classify");
    assert_eq!(refusal.store, CompatibilityStore::HistoryTree);
    assert_eq!(refusal.found, Some(2));

    let weight_mismatch = serde_json::to_vec(&mutated(&encoded, |value| {
        value["nodes"][0]["encodedWeight"] = Value::from(999)
    }))
    .unwrap();
    let error = persistence
        .load(&history, &weight_mismatch)
        .expect_err("a weight mismatch must not load");
    assert_eq!(
        error.future_schema_refusal(),
        None,
        "a payload fault is not a version problem and must not be reported as one"
    );

    drop(live);
}

/// Card 182. The operator decision, enforced: an envelope written before
/// `recorded_at` existed has no such field, and must load as `None` rather
/// than failing. Nothing backfills a time the system never observed.
#[test]
fn an_envelope_without_recorded_at_loads_as_none() {
    let (graph, encoded) = encoded_v1();
    let document: Value = serde_json::from_slice(&encoded).expect("fixture JSON");
    assert!(
        document["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .all(|node| node.get("recordedAt").is_none()),
        "a history whose host never stamped writes the field nowhere"
    );

    let loaded = persistence_v1()
        .load(graph.history_id(), &encoded)
        .expect("an envelope without the field still loads");
    assert!(
        loaded
            .history()
            .nodes()
            .all(|node| node.metadata().recorded_at().is_none())
    );
}

/// A supplied stamp survives encode and decode unchanged, and is the only
/// difference in the envelope.
#[test]
fn a_supplied_recorded_at_round_trips() {
    let stamped = HistoryRecordedAt::from_epoch_millis(1_765_432_100_000);
    let mut graph = ForkHistory::new(
        history_id("history:stamped"),
        branch_id("branch:main"),
        branch_metadata("Main", true),
    );
    let payload = mutation(1, b"alpha");
    let encoded_weight = mutation_weight(1, &payload.body);
    graph
        .record_applied(ForkRecord::new(
            graph.revision(),
            entry_id("entry:a"),
            metadata("Stamped").with_recorded_at(stamped),
            encoded_weight,
            payload,
            None,
        ))
        .expect("record stamped entry");

    let encoded = persistence_v1().encode(&graph).expect("encode stamped");
    let document: Value = serde_json::from_slice(&encoded).expect("stamped JSON");
    assert_eq!(
        document["nodes"][0]["recordedAt"].as_u64(),
        Some(stamped.epoch_millis())
    );

    let loaded = persistence_v1()
        .load(graph.history_id(), &encoded)
        .expect("load stamped");
    assert_eq!(
        loaded
            .history()
            .nodes()
            .next()
            .expect("one node")
            .metadata()
            .recorded_at(),
        Some(stamped)
    );
}

// Property tests for the dense-envelope decode path (card 213). Two
// properties run 64 fixed cases each: arbitrary bytes never panic
// `ForkPersistence::load` and load deterministically; a valid envelope
// mutated by bit flips, truncation, and lies in numeric length or sequence
// fields fails classified (a typed `ForkLoadError`) or loads, and a loaded
// graph re-encodes and re-loads cleanly. Measured cost: well under one
// second for both properties.

const FUZZ_CASES: u32 = 64;

/// Writes a lying numeric value into one of the envelope's weight, sequence,
/// or revision fields — the fields a corrupt envelope uses to disagree with
/// its own payload bytes.
fn apply_numeric_lie(document: &mut Value, field: usize, value: u64) {
    let node_index = usize::try_from(value).unwrap_or(0) % 3;
    match field % 5 {
        0 => document["nextSequence"] = Value::from(value),
        1 => document["revision"] = Value::from(value),
        2 => document["nodes"][node_index]["encodedWeight"] = Value::from(value),
        3 => document["nodes"][node_index]["sequence"] = Value::from(value),
        _ => document["nodes"][node_index]["committedRevision"] = Value::from(value),
    }
}

/// JSON-shaped byte strings, occasionally prefixed with a valid envelope
/// fragment, so cases reach past JSON rejection into header checks.
fn hostile_bytes() -> impl Strategy<Value = Vec<u8>> {
    (
        prop::collection::vec(
            prop_oneof![
                3 => prop::sample::select(b"{}\":,[]0-9a-z".to_vec()),
                1 => any::<u8>(),
            ],
            0..=300,
        ),
        any::<bool>(),
    )
        .prop_map(|(bytes, prefix)| {
            if prefix {
                let (_, valid) = encoded_v1();
                let take = bytes.len().min(valid.len());
                let mut prefixed = valid[..take].to_vec();
                prefixed.extend_from_slice(&bytes);
                prefixed
            } else {
                bytes
            }
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(FUZZ_CASES))]

    /// Arbitrary bytes never panic the loader, and every outcome is
    /// deterministic: two loads of the same bytes agree exactly.
    #[test]
    fn arbitrary_bytes_load_deterministically_without_panic(bytes in hostile_bytes()) {
        let history = history_id("history:fork-fixture");
        let first = persistence_v1().load(&history, &bytes);
        let second = persistence_v1().load(&history, &bytes);
        prop_assert_eq!(format!("{first:?}"), format!("{second:?}"));
    }

    /// A valid envelope corrupted by numeric-field lies, bit flips, and
    /// truncation never panics the loader. When a mutated envelope still
    /// loads, the resulting graph re-encodes and re-loads cleanly — a load
    /// must never produce state the encoder cannot persist again.
    #[test]
    fn mutated_valid_envelopes_fail_classified_or_reencode(
        lies in prop::collection::vec((any::<usize>(), any::<u64>()), 0..=2),
        flips in prop::collection::vec((any::<usize>(), 0..8_u8), 0..=3),
        truncation in prop::option::of(any::<usize>()),
    ) {
        let (graph, mut bytes) = encoded_v1();
        for (field, value) in lies {
            if let Ok(mut document) = serde_json::from_slice::<Value>(&bytes) {
                apply_numeric_lie(&mut document, field, value);
                bytes = serde_json::to_vec(&document).expect("lie re-serialization");
            }
        }
        for (offset, bit) in flips {
            let offset = offset % bytes.len().max(1);
            if let Some(byte) = bytes.get_mut(offset) {
                *byte ^= 1 << bit;
            }
        }
        if let Some(len) = truncation {
            bytes.truncate(len % (bytes.len() + 1));
        }

        let first = persistence_v1().load(graph.history_id(), &bytes);
        let second = persistence_v1().load(graph.history_id(), &bytes);
        prop_assert_eq!(format!("{first:?}"), format!("{second:?}"));
        if let Ok(loaded) = first {
            let reencoded = persistence_v1()
                .encode(loaded.history())
                .expect("a loaded graph must re-encode");
            persistence_v1()
                .load(graph.history_id(), &reencoded)
                .expect("a re-encoded graph must re-load");
        }
    }
}
