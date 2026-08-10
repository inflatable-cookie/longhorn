use std::convert::Infallible;

use longhorn_core::HistoryPlanId;
use longhorn_history::{
    AppliedHistoryRecord, HistoryCoalesce, HistoryCoalesceContext, HistoryCommittedTransition,
    HistoryLimits, HistoryNavigationPlan, HistoryNavigationRequest, HistoryNavigationTarget,
    HistoryNavigationTransaction, HistoryNavigationTransactionFailure, HistoryPayloadCodec,
    HistoryPayloadCodecFamily, HistoryPayloadCodecVersion, HistoryPersistence,
    HistoryPersistenceLimits, HistoryPolicy, LinearHistory,
};

use crate::{
    pulse_shaped::{PulseFixtureMutation, PulseFixturePolicy, PulseFixturePolicyError, rename},
    support::*,
};

struct PersistedPulsePolicy;

impl HistoryPolicy<PulseFixtureMutation> for PersistedPulsePolicy {
    type Error = PulseFixturePolicyError;

    fn inverse(&self, payload: &PulseFixtureMutation) -> Result<PulseFixtureMutation, Self::Error> {
        PulseFixturePolicy.inverse(payload)
    }

    fn is_noop(&self, payload: &PulseFixtureMutation) -> bool {
        PulseFixturePolicy.is_noop(payload)
    }

    fn encoded_weight(&self, payload: &PulseFixtureMutation) -> Result<u64, Self::Error> {
        u64::try_from(
            serde_json::to_vec(payload)
                .map_err(|_| PulseFixturePolicyError)?
                .len(),
        )
        .map_err(|_| PulseFixturePolicyError)
    }

    fn coalesce(
        &self,
        previous: &PulseFixtureMutation,
        incoming: &PulseFixtureMutation,
        context: HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<PulseFixtureMutation>, Self::Error> {
        PulseFixturePolicy.coalesce(previous, incoming, context)
    }
}

struct PulseCodec {
    family: HistoryPayloadCodecFamily,
}

impl PulseCodec {
    fn new() -> Self {
        Self {
            family: HistoryPayloadCodecFamily::new("pulse-fixture").unwrap(),
        }
    }
}

impl HistoryPayloadCodec<PulseFixtureMutation> for PulseCodec {
    type Error = serde_json::Error;

    fn family(&self) -> &HistoryPayloadCodecFamily {
        &self.family
    }

    fn version(&self) -> HistoryPayloadCodecVersion {
        HistoryPayloadCodecVersion::new(1)
    }

    fn encode(&self, payload: &PulseFixtureMutation) -> Result<Vec<u8>, Self::Error> {
        serde_json::to_vec(payload)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PulseFixtureMutation, Self::Error> {
        serde_json::from_slice(bytes)
    }
}

#[derive(Clone)]
struct LoopholeJournalRecord {
    product_revision: u64,
    command: AppliedHistoryRecord<PulseFixtureMutation>,
    expected_transition: HistoryCommittedTransition,
}

#[derive(Default)]
struct DisposableJournal {
    durable_suffix: Vec<LoopholeJournalRecord>,
    fail_next: bool,
}

impl DisposableJournal {
    fn append(&mut self, record: LoopholeJournalRecord) -> Result<(), &'static str> {
        if self.fail_next {
            self.fail_next = false;
            return Err("injected journal failure");
        }
        self.durable_suffix.push(record);
        Ok(())
    }
}

struct LoopholeProjectSnapshot {
    product_revision: u64,
    history_bytes: Vec<u8>,
}

struct SuccessfulTransaction;

impl HistoryNavigationTransaction<PulseFixtureMutation> for SuccessfulTransaction {
    type Error = Infallible;

    fn apply(
        &mut self,
        _: &HistoryNavigationPlan<PulseFixtureMutation>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        Ok(())
    }
}

#[test]
fn pulse_shaped_snapshot_and_disposable_journal_keep_policy_and_durability_external() {
    let policy = PersistedPulsePolicy;
    let persistence = HistoryPersistence::without_structural_migration(
        PulseCodec::new(),
        HistoryPersistenceLimits::new(256 * 1_024).unwrap(),
    );
    let mut history = LinearHistory::new(
        history_id("history:pulse-persisted"),
        HistoryLimits::default(),
    );

    history
        .record_applied(
            record(
                0,
                "entry:rename",
                metadata("Rename track", "track:rename"),
                rename(1, "Drums", "Kit"),
            ),
            &policy,
        )
        .unwrap();
    history
        .record_applied(
            record(
                1,
                "entry:rename-2",
                metadata("Rename track again", "track:rename"),
                rename(1, "Kit", "Beats"),
            ),
            &policy,
        )
        .unwrap();

    let snapshot = LoopholeProjectSnapshot {
        product_revision: 41,
        history_bytes: persistence.encode(&history).unwrap(),
    };
    let mut journal = DisposableJournal::default();
    let suffix_command = record(
        2,
        "entry:delete",
        metadata("Delete track", "track:delete"),
        PulseFixtureMutation::DeleteTrack {
            track_id: 2,
            snapshot: "Keys".to_owned(),
        },
    );
    let suffix_result = history
        .record_applied(suffix_command.clone(), &policy)
        .unwrap();
    journal
        .append(LoopholeJournalRecord {
            product_revision: 42,
            command: suffix_command,
            expected_transition: suffix_result.transition().unwrap().clone(),
        })
        .unwrap();

    let failed_journal_command = record(
        3,
        "entry:restore",
        metadata("Restore track", "track:restore"),
        PulseFixtureMutation::RestoreTrack {
            track_id: 3,
            snapshot: "FX".to_owned(),
        },
    );
    let committed_before_journal_failure = history
        .record_applied(failed_journal_command.clone(), &policy)
        .unwrap()
        .transition()
        .unwrap()
        .clone();
    journal.fail_next = true;
    assert!(
        journal
            .append(LoopholeJournalRecord {
                product_revision: 43,
                command: failed_journal_command,
                expected_transition: committed_before_journal_failure.clone(),
            })
            .is_err()
    );
    assert_eq!(
        committed_before_journal_failure.committed_revision(),
        history.revision()
    );
    assert_eq!(journal.durable_suffix.len(), 1);

    let loaded = persistence
        .load(
            &history_id("history:pulse-persisted"),
            &snapshot.history_bytes,
            &policy,
        )
        .unwrap();
    let (mut recovered, _) = loaded.into_parts();
    let mut recovered_product_revision = snapshot.product_revision;
    for durable in &journal.durable_suffix {
        let result = recovered
            .record_applied(durable.command.clone(), &policy)
            .unwrap();
        assert_eq!(result.transition().unwrap(), &durable.expected_transition);
        recovered_product_revision = durable.product_revision;
    }
    assert_eq!(recovered_product_revision, 42);
    assert_eq!(recovered.revision().get(), 3);
    assert_eq!(recovered.applied().len(), 2);

    let undo = recovered
        .plan_navigation(
            HistoryNavigationRequest::new(
                HistoryPlanId::new("plan:cross-session-undo").unwrap(),
                recovered.revision(),
                HistoryNavigationTarget::Undo,
            ),
            &policy,
        )
        .unwrap();
    recovered
        .execute_navigation(undo, &mut SuccessfulTransaction)
        .unwrap();
    assert_eq!(
        recovered.current().unwrap().payload(),
        &rename(1, "Drums", "Beats")
    );
}
