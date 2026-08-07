use std::{convert::Infallible, error::Error, fmt};

use longhorn_core::{CompatibilityStore, FutureSchemaRefused};
use longhorn_history::{
    HistoryCoalesce, HistoryCoalesceContext, HistoryDiscardReason, HistoryEncodeError,
    HistoryLoadError, HistoryLoadOutcome, HistoryNavigationPlan, HistoryNavigationRequest,
    HistoryNavigationTarget, HistoryNavigationTransaction, HistoryNavigationTransactionFailure,
    HistoryPayloadCodec, HistoryPayloadCodecFamily, HistoryPayloadCodecVersion,
    HistoryPayloadMigrationStep, HistoryPayloadMigrationTarget, HistoryPersistence,
    HistoryPersistenceLimits, HistoryPolicy, HistoryStructuralMigration,
    HistoryStructuralMigrationStep, HistoryStructuralMigrationTarget, LinearHistory,
    discard_persisted_history,
};
use serde_json::Value;

use crate::support::*;

#[derive(Clone, Debug, Eq, PartialEq)]
enum CounterMutation {
    Set { before: i32, after: i32 },
}

#[derive(Clone, Copy)]
struct CounterPolicy {
    encoded_weight: u64,
}

impl HistoryPolicy<CounterMutation> for CounterPolicy {
    type Error = Infallible;

    fn inverse(&self, payload: &CounterMutation) -> Result<CounterMutation, Self::Error> {
        Ok(match payload {
            CounterMutation::Set { before, after } => CounterMutation::Set {
                before: *after,
                after: *before,
            },
        })
    }

    fn is_noop(&self, payload: &CounterMutation) -> bool {
        matches!(payload, CounterMutation::Set { before, after } if before == after)
    }

    fn encoded_weight(&self, _: &CounterMutation) -> Result<u64, Self::Error> {
        Ok(self.encoded_weight)
    }

    fn coalesce(
        &self,
        _: &CounterMutation,
        _: &CounterMutation,
        _: HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<CounterMutation>, Self::Error> {
        Ok(HistoryCoalesce::KeepSeparate)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CounterCodecError {
    InvalidLength,
    InvalidChecksum,
}

impl fmt::Display for CounterCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength => formatter.write_str("counter payload length is invalid"),
            Self::InvalidChecksum => formatter.write_str("counter payload checksum is invalid"),
        }
    }
}

impl Error for CounterCodecError {}

#[derive(Clone)]
struct CounterCodec {
    family: HistoryPayloadCodecFamily,
    version: HistoryPayloadCodecVersion,
}

impl CounterCodec {
    fn version_one() -> Self {
        Self {
            family: HistoryPayloadCodecFamily::new("fixture.counter").unwrap(),
            version: HistoryPayloadCodecVersion::new(1),
        }
    }

    fn version_two() -> Self {
        Self {
            family: HistoryPayloadCodecFamily::new("fixture.counter").unwrap(),
            version: HistoryPayloadCodecVersion::new(2),
        }
    }
}

impl HistoryPayloadCodec<CounterMutation> for CounterCodec {
    type Error = CounterCodecError;

    fn family(&self) -> &HistoryPayloadCodecFamily {
        &self.family
    }

    fn version(&self) -> HistoryPayloadCodecVersion {
        self.version
    }

    fn encode(&self, payload: &CounterMutation) -> Result<Vec<u8>, Self::Error> {
        let CounterMutation::Set { before, after } = payload;
        let mut bytes = Vec::with_capacity(if self.version.get() == 1 { 8 } else { 9 });
        bytes.extend_from_slice(&before.to_le_bytes());
        bytes.extend_from_slice(&after.to_le_bytes());
        if self.version.get() == 2 {
            bytes.push(checksum(&bytes));
        }
        Ok(bytes)
    }

    fn decode(&self, bytes: &[u8]) -> Result<CounterMutation, Self::Error> {
        let expected = if self.version.get() == 1 { 8 } else { 9 };
        if bytes.len() != expected {
            return Err(CounterCodecError::InvalidLength);
        }
        if self.version.get() == 2 && bytes[8] != checksum(&bytes[..8]) {
            return Err(CounterCodecError::InvalidChecksum);
        }
        let before = i32::from_le_bytes(
            bytes[..4]
                .try_into()
                .map_err(|_| CounterCodecError::InvalidLength)?,
        );
        let after = i32::from_le_bytes(
            bytes[4..8]
                .try_into()
                .map_err(|_| CounterCodecError::InvalidLength)?,
        );
        Ok(CounterMutation::Set { before, after })
    }

    fn migrate_one(
        &self,
        from: HistoryPayloadCodecVersion,
        mut bytes: Vec<u8>,
        target: HistoryPayloadMigrationTarget<'_>,
    ) -> Result<Option<HistoryPayloadMigrationStep>, Self::Error> {
        if from.get() == 1 && target.version().get() == 2 {
            if bytes.len() != 8 {
                return Err(CounterCodecError::InvalidLength);
            }
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

struct SuccessfulTransaction;

impl HistoryNavigationTransaction<CounterMutation> for SuccessfulTransaction {
    type Error = Infallible;

    fn apply(
        &mut self,
        _: &HistoryNavigationPlan<CounterMutation>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        Ok(())
    }
}

fn persistence_limits() -> HistoryPersistenceLimits {
    HistoryPersistenceLimits::new(64 * 1_024).unwrap()
}

fn persisted_history() -> LinearHistory<CounterMutation> {
    let limits = longhorn_history::HistoryLimits::new(10, 1_024, 64).unwrap();
    let policy = CounterPolicy { encoded_weight: 8 };
    let mut history = LinearHistory::new(history_id("history:counter"), limits);
    history
        .record_applied(
            record(
                0,
                "entry:counter-1",
                metadata("Set counter to one", "fixture:counter"),
                CounterMutation::Set {
                    before: 0,
                    after: 1,
                },
            ),
            &policy,
        )
        .unwrap();
    history
        .record_applied(
            record(
                1,
                "entry:counter-2",
                metadata("Set counter to two", "fixture:counter"),
                CounterMutation::Set {
                    before: 1,
                    after: 2,
                },
            ),
            &policy,
        )
        .unwrap();
    let plan = history
        .plan_navigation(
            HistoryNavigationRequest::new(
                longhorn_core::HistoryPlanId::new("plan:counter-undo").unwrap(),
                history.revision(),
                HistoryNavigationTarget::Undo,
            ),
            &policy,
        )
        .unwrap();
    history
        .execute_navigation(plan, &mut SuccessfulTransaction)
        .unwrap();
    history
}

#[test]
fn golden_envelope_round_trips_complete_linear_state() {
    let history = persisted_history();
    let persistence = HistoryPersistence::without_structural_migration(
        CounterCodec::version_one(),
        persistence_limits(),
    );
    let bytes = persistence.encode(&history).unwrap();
    assert_eq!(
        String::from_utf8(bytes.clone()).unwrap(),
        include_str!("../../fixtures/history/linear-v1.json").trim_end()
    );

    let loaded = persistence
        .load(
            history.history_id(),
            &bytes,
            &CounterPolicy { encoded_weight: 8 },
        )
        .unwrap();
    assert_eq!(loaded.receipt().outcome(), HistoryLoadOutcome::Preserved);
    assert_eq!(
        loaded.history().clone().into_state(),
        history.clone().into_state()
    );
    assert!(matches!(
        loaded.receipt().transition().kind(),
        longhorn_history::HistoryCommittedTransitionKind::Imported {
            source_structural_version: 1,
            structural_version: 1,
            source_payload_codec_version: 1,
            payload_codec_version: 1,
            applied_entries: 1,
            future_entries: 1,
            ..
        }
    ));
}

#[test]
fn future_corrupt_unbounded_and_foreign_sources_reject_without_live_mutation() {
    let live = persisted_history();
    let before = live.clone();
    let persistence = HistoryPersistence::without_structural_migration(
        CounterCodec::version_one(),
        persistence_limits(),
    );
    let encoded = persistence.encode(&live).unwrap();
    let policy = CounterPolicy { encoded_weight: 8 };

    let mut future_structural: Value = serde_json::from_slice(&encoded).unwrap();
    future_structural["structuralVersion"] = Value::from(2);
    assert!(matches!(
        persistence.load(
            live.history_id(),
            &serde_json::to_vec(&future_structural).unwrap(),
            &policy
        ),
        Err(HistoryLoadError::FutureStructuralVersion { .. })
    ));

    let mut future_payload: Value = serde_json::from_slice(&encoded).unwrap();
    future_payload["payloadCodec"]["version"] = Value::from(2);
    assert!(matches!(
        persistence.load(
            live.history_id(),
            &serde_json::to_vec(&future_payload).unwrap(),
            &policy
        ),
        Err(HistoryLoadError::FuturePayloadCodecVersion { .. })
    ));

    let mut corrupt: Value = serde_json::from_slice(&encoded).unwrap();
    corrupt["entries"][0]["payload"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(matches!(
        persistence.load(
            live.history_id(),
            &serde_json::to_vec(&corrupt).unwrap(),
            &policy
        ),
        Err(HistoryLoadError::PayloadWeightMismatch { .. })
    ));

    let mut bad_position: Value = serde_json::from_slice(&encoded).unwrap();
    bad_position["currentPosition"] = Value::from(99);
    assert!(matches!(
        persistence.load(
            live.history_id(),
            &serde_json::to_vec(&bad_position).unwrap(),
            &policy
        ),
        Err(HistoryLoadError::InvalidCurrentPosition { .. })
    ));

    let mut unknown_field: Value = serde_json::from_slice(&encoded).unwrap();
    unknown_field["unexpected"] = Value::Bool(true);
    assert!(matches!(
        persistence.load(
            live.history_id(),
            &serde_json::to_vec(&unknown_field).unwrap(),
            &policy
        ),
        Err(HistoryLoadError::InvalidEnvelope(_))
    ));

    let mut unbounded_family: Value = serde_json::from_slice(&encoded).unwrap();
    unbounded_family["payloadCodec"]["family"] = Value::from("x".repeat(129));
    assert!(matches!(
        persistence.load(
            live.history_id(),
            &serde_json::to_vec(&unbounded_family).unwrap(),
            &policy
        ),
        Err(HistoryLoadError::InvalidEnvelope(_))
    ));

    assert!(matches!(
        persistence.load(&history_id("history:other"), &encoded, &policy),
        Err(HistoryLoadError::ForeignHistory { .. })
    ));
    let bounded = HistoryPersistence::without_structural_migration(
        CounterCodec::version_one(),
        HistoryPersistenceLimits::new(32).unwrap(),
    );
    assert!(matches!(
        bounded.load(live.history_id(), &encoded, &policy),
        Err(HistoryLoadError::EnvelopeTooLarge { .. })
    ));
    assert_eq!(live, before);
}

#[derive(Clone, Copy)]
struct VersionZeroStructuralMigration;

impl HistoryStructuralMigration for VersionZeroStructuralMigration {
    type Error = Infallible;

    fn migrate_one(
        &self,
        from: u32,
        mut document: Value,
        target: HistoryStructuralMigrationTarget,
    ) -> Result<Option<HistoryStructuralMigrationStep>, Self::Error> {
        if from == 0 && target.version() == 1 {
            document["structuralVersion"] = Value::from(1);
            Ok(Some(HistoryStructuralMigrationStep::new(1, document)))
        } else {
            Ok(None)
        }
    }
}

#[test]
fn structural_and_payload_versions_migrate_independently_then_revalidate() {
    let history = persisted_history();
    let source = HistoryPersistence::without_structural_migration(
        CounterCodec::version_one(),
        persistence_limits(),
    );
    let mut old: Value = serde_json::from_slice(&source.encode(&history).unwrap()).unwrap();
    old["structuralVersion"] = Value::from(0);
    let old = serde_json::to_vec(&old).unwrap();

    let current = HistoryPersistence::new(
        CounterCodec::version_two(),
        VersionZeroStructuralMigration,
        persistence_limits(),
    );
    let loaded = current
        .load(
            history.history_id(),
            &old,
            &CounterPolicy { encoded_weight: 9 },
        )
        .unwrap();
    assert_eq!(
        loaded.receipt().outcome(),
        HistoryLoadOutcome::Migrated {
            structural: true,
            payload: true,
        }
    );
    assert_eq!(loaded.history().applied()[0].encoded_weight(), 9);
    assert_eq!(loaded.history().future()[0].encoded_weight(), 9);

    let no_structural_migration = HistoryPersistence::without_structural_migration(
        CounterCodec::version_two(),
        persistence_limits(),
    );
    assert!(matches!(
        no_structural_migration.load(
            history.history_id(),
            &old,
            &CounterPolicy { encoded_weight: 9 }
        ),
        Err(HistoryLoadError::MissingStructuralMigration { from: 0 })
    ));
}

#[test]
fn incompatible_source_requires_an_explicit_discard_receipt() {
    let history = persisted_history();
    let persistence = HistoryPersistence::without_structural_migration(
        CounterCodec::version_one(),
        persistence_limits(),
    );
    let mut corrupt: Value =
        serde_json::from_slice(&persistence.encode(&history).unwrap()).unwrap();
    corrupt["payloadCodec"]["family"] = Value::from("fixture.other");
    assert!(matches!(
        persistence.load(
            history.history_id(),
            &serde_json::to_vec(&corrupt).unwrap(),
            &CounterPolicy { encoded_weight: 8 }
        ),
        Err(HistoryLoadError::ForeignPayloadCodecFamily { .. })
    ));

    let recovery = discard_persisted_history::<CounterMutation>(
        history_id("history:counter"),
        history.limits(),
        history.navigation_limits(),
        history.projection_limits(),
        HistoryDiscardReason::IncompatibleSource,
    );
    assert!(recovery.history().applied().is_empty());
    assert!(recovery.history().future().is_empty());
    assert_eq!(
        recovery.receipt().reason(),
        HistoryDiscardReason::IncompatibleSource
    );
    assert!(matches!(
        recovery.receipt().transition().kind(),
        longhorn_history::HistoryCommittedTransitionKind::DiscardedPersistence {
            reason: HistoryDiscardReason::IncompatibleSource
        }
    ));
}

#[test]
fn encode_rejects_a_policy_measurement_that_is_not_codec_bytes() {
    let mut history = LinearHistory::new(
        history_id("history:counter"),
        longhorn_history::HistoryLimits::default(),
    );
    history
        .record_applied(
            record(
                0,
                "entry:counter-1",
                metadata("Set counter", "fixture:counter"),
                CounterMutation::Set {
                    before: 0,
                    after: 1,
                },
            ),
            &CounterPolicy { encoded_weight: 7 },
        )
        .unwrap();
    let persistence = HistoryPersistence::without_structural_migration(
        CounterCodec::version_one(),
        persistence_limits(),
    );
    assert!(matches!(
        persistence.encode(&history),
        Err(HistoryEncodeError::PayloadWeightMismatch {
            recorded: 7,
            actual: 8,
            ..
        })
    ));
}

#[test]
fn future_versions_classify_for_the_update_surface() {
    // Both version axes reach a channel rejoin independently: a newer build
    // can advance the structural envelope, the payload codec, or both. The
    // update surface has to recognise either without knowing which.
    let live = persisted_history();
    let persistence = HistoryPersistence::without_structural_migration(
        CounterCodec::version_one(),
        persistence_limits(),
    );
    let encoded = persistence.encode(&live).unwrap();
    let policy = CounterPolicy { encoded_weight: 8 };

    let mut future_structural: Value = serde_json::from_slice(&encoded).unwrap();
    future_structural["structuralVersion"] = Value::from(2);
    let error = persistence
        .load(
            live.history_id(),
            &serde_json::to_vec(&future_structural).unwrap(),
            &policy,
        )
        .expect_err("a future structural version must not load");
    let refusal = error
        .future_schema_refusal()
        .expect("a future structural version must classify");
    assert_eq!(refusal.store, CompatibilityStore::History);
    assert_eq!(refusal.found, Some(2));
    assert_eq!(refusal.supported, Some(1));

    let mut future_payload: Value = serde_json::from_slice(&encoded).unwrap();
    future_payload["payloadCodec"]["version"] = Value::from(2);
    let error = persistence
        .load(
            live.history_id(),
            &serde_json::to_vec(&future_payload).unwrap(),
            &policy,
        )
        .expect_err("a future payload codec version must not load");
    let refusal = error
        .future_schema_refusal()
        .expect("a future payload codec version must classify");
    assert_eq!(refusal.store, CompatibilityStore::History);
    assert_eq!(refusal.found, Some(2));
    assert_eq!(refusal.supported, Some(1));

    let mut bad_position: Value = serde_json::from_slice(&encoded).unwrap();
    bad_position["currentPosition"] = Value::from(99);
    let error = persistence
        .load(
            live.history_id(),
            &serde_json::to_vec(&bad_position).unwrap(),
            &policy,
        )
        .expect_err("an invalid position must not load");
    assert_eq!(
        error.future_schema_refusal(),
        None,
        "a structural fault is not a version problem and must not be reported as one"
    );
}
