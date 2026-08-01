//! Strict graph persistence and independent migration evidence.

mod support;

use std::convert::Infallible;

use longhorn_history::{HistoryPayloadCodecFamily, HistoryPayloadCodecVersion};
use longhorn_history_tree_prototype::{
    ForkBranchSeed, ForkCheckpointId, ForkHistory, ForkLoadError, ForkLoadOutcome,
    ForkNavigationTarget, ForkPayloadCodec, ForkPayloadMigrationStep, ForkPayloadMigrationTarget,
    ForkPersistence, ForkPersistenceValidationError, ForkStructuralMigration,
    ForkStructuralMigrationStep, ForkStructuralMigrationTarget, NoForkStructuralMigration,
};
use serde_json::Value;

use support::{
    Delta, DeltaPolicy, ModelTransaction, TransactionMode, branch_id, branch_metadata, entry_id,
    history, plan_id, record,
};

#[derive(Clone)]
struct DeltaCodec {
    family: HistoryPayloadCodecFamily,
    version: HistoryPayloadCodecVersion,
}

impl DeltaCodec {
    fn current() -> Self {
        Self {
            family: HistoryPayloadCodecFamily::new("fixture.delta").unwrap(),
            version: HistoryPayloadCodecVersion::new(2),
        }
    }
}

impl ForkPayloadCodec<Delta> for DeltaCodec {
    type Error = &'static str;

    fn family(&self) -> &HistoryPayloadCodecFamily {
        &self.family
    }

    fn version(&self) -> HistoryPayloadCodecVersion {
        self.version
    }

    fn encode(&self, payload: &Delta) -> Result<Vec<u8>, Self::Error> {
        Ok(payload.0.to_le_bytes().to_vec())
    }

    fn decode(&self, bytes: &[u8]) -> Result<Delta, Self::Error> {
        let bytes: [u8; 8] = bytes.try_into().map_err(|_| "invalid delta bytes")?;
        Ok(Delta(i64::from_le_bytes(bytes)))
    }

    fn migrate_one(
        &self,
        from: HistoryPayloadCodecVersion,
        bytes: Vec<u8>,
        target: ForkPayloadMigrationTarget<'_>,
    ) -> Result<Option<ForkPayloadMigrationStep>, Self::Error> {
        assert_eq!(target.family(), "fixture.delta");
        assert_eq!(target.version(), HistoryPayloadCodecVersion::new(2));
        if from != HistoryPayloadCodecVersion::new(1) {
            return Ok(None);
        }
        let bytes: [u8; 8] = bytes.try_into().map_err(|_| "invalid v1 delta bytes")?;
        Ok(Some(ForkPayloadMigrationStep::new(
            HistoryPayloadCodecVersion::new(2),
            i64::from_be_bytes(bytes).to_le_bytes().to_vec(),
        )))
    }
}

#[derive(Clone, Copy)]
struct V0Migration;

impl ForkStructuralMigration for V0Migration {
    type Error = Infallible;

    fn migrate_one(
        &self,
        from: u32,
        mut document: Value,
        target: ForkStructuralMigrationTarget,
    ) -> Result<Option<ForkStructuralMigrationStep>, Self::Error> {
        assert_eq!(target.family(), "longhorn.private.history-tree");
        assert_eq!(target.version(), 1);
        if from != 0 {
            return Ok(None);
        }
        let object = document.as_object_mut().expect("fixture object");
        object.insert("structuralVersion".to_owned(), Value::from(1));
        object.insert("checkpoints".to_owned(), Value::Array(Vec::new()));
        Ok(Some(ForkStructuralMigrationStep::new(1, document)))
    }
}

fn graph() -> ForkHistory<Delta> {
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
            branch_metadata("Alternate", true),
        )),
    );
    history
        .register_checkpoint(
            history.revision(),
            ForkCheckpointId::new("checkpoint:b").unwrap(),
            Some(entry_id("entry:b")),
            "consumer://snapshot/b".to_owned(),
        )
        .unwrap();
    history
}

fn encoded_document() -> (ForkHistory<Delta>, Value) {
    let history = graph();
    let bytes = ForkPersistence::without_structural_migration(DeltaCodec::current())
        .encode(&history)
        .unwrap();
    (history, serde_json::from_slice(&bytes).unwrap())
}

#[test]
fn current_graph_round_trips_exactly() {
    let history = graph();
    let persistence = ForkPersistence::without_structural_migration(DeltaCodec::current());
    let bytes = persistence.encode(&history).unwrap();
    let loaded = persistence.load(history.history_id(), &bytes).unwrap();

    assert_eq!(loaded.receipt().outcome(), ForkLoadOutcome::Preserved);
    assert_eq!(loaded.history(), &history);
    assert_eq!(persistence.encode(loaded.history()).unwrap(), bytes);
}

#[test]
fn corrupt_future_and_unknown_sources_never_replace_live_graph() {
    let persistence = ForkPersistence::without_structural_migration(DeltaCodec::current());
    let (live, document) = encoded_document();
    let source = live.clone();

    let mut corrupt = document.clone();
    corrupt["nodes"][0]["parentEntryId"] = Value::from("entry:absent");
    let error = persistence
        .load::<Delta>(live.history_id(), &serde_json::to_vec(&corrupt).unwrap())
        .unwrap_err();
    assert!(matches!(
        error,
        ForkLoadError::Validation(ForkPersistenceValidationError::InvalidParent)
    ));

    let mut future = document.clone();
    future["structuralVersion"] = Value::from(2);
    assert!(matches!(
        persistence
            .load::<Delta>(live.history_id(), &serde_json::to_vec(&future).unwrap())
            .unwrap_err(),
        ForkLoadError::FutureStructuralVersion { .. }
    ));

    let mut unknown = document;
    unknown["invented"] = Value::Bool(true);
    assert!(matches!(
        persistence
            .load::<Delta>(live.history_id(), &serde_json::to_vec(&unknown).unwrap())
            .unwrap_err(),
        ForkLoadError::InvalidEnvelope(_)
    ));
    assert_eq!(live, source);
}

#[test]
fn structure_and_payload_migrate_independently() {
    let (history, mut document) = encoded_document();
    document["structuralVersion"] = Value::from(0);
    document.as_object_mut().unwrap().remove("checkpoints");
    document["payloadCodec"]["version"] = Value::from(1);
    for node in document["nodes"].as_array_mut().unwrap() {
        let little = node["payload"]
            .as_array()
            .unwrap()
            .iter()
            .map(|byte| byte.as_u64().unwrap() as u8)
            .collect::<Vec<_>>();
        let value = i64::from_le_bytes(little.try_into().unwrap());
        node["payload"] = serde_json::to_value(value.to_be_bytes()).unwrap();
    }
    let persistence = ForkPersistence::new(DeltaCodec::current(), V0Migration);
    let loaded = persistence
        .load::<Delta>(
            history.history_id(),
            &serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();

    assert_eq!(
        loaded.receipt().outcome(),
        ForkLoadOutcome::Migrated {
            structural: true,
            payload: true,
        }
    );
    assert_eq!(loaded.history().retained_entry_count(), 4);
    assert_eq!(loaded.history().checkpoints().count(), 0);
    assert_eq!(
        loaded
            .history()
            .node(&entry_id("entry:d"))
            .unwrap()
            .payload(),
        &Delta(4)
    );
}

#[test]
fn foreign_and_future_payload_codecs_fail_visibly() {
    let persistence = ForkPersistence::<_, NoForkStructuralMigration>::without_structural_migration(
        DeltaCodec::current(),
    );
    let (history, document) = encoded_document();

    let mut foreign = document.clone();
    foreign["payloadCodec"]["family"] = Value::from("fixture.foreign");
    assert!(matches!(
        persistence
            .load::<Delta>(history.history_id(), &serde_json::to_vec(&foreign).unwrap())
            .unwrap_err(),
        ForkLoadError::ForeignPayloadCodecFamily
    ));

    let mut future = document;
    future["payloadCodec"]["version"] = Value::from(3);
    assert!(matches!(
        persistence
            .load::<Delta>(history.history_id(), &serde_json::to_vec(&future).unwrap())
            .unwrap_err(),
        ForkLoadError::FuturePayloadCodecVersion { .. }
    ));
}
