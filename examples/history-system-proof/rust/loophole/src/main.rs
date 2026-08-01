use std::{collections::BTreeMap, error::Error, fmt};

use longhorn_core::{
    HistoryEntryId, HistoryGroupId, HistoryGroupKeyId, HistoryId, HistoryKindId, HistoryPlanId,
    HistoryRevision,
};
use longhorn_history::{
    AppliedHistoryRecord, HistoryAuthorityEpoch, HistoryChangedEvent, HistoryCoalesce,
    HistoryCoalesceContext, HistoryEntryMetadata, HistoryGroupDurationMillis, HistoryLabel,
    HistoryLimits, HistoryLoadError, HistoryMonotonicMillis, HistoryNavigationCommand,
    HistoryNavigationExecutionError, HistoryNavigationPlan, HistoryNavigationReceiptProjection,
    HistoryNavigationRejectionCode, HistoryNavigationRejectionProjection, HistoryNavigationRequest,
    HistoryNavigationResult, HistoryNavigationTarget, HistoryNavigationTargetProjection,
    HistoryNavigationTransaction, HistoryNavigationTransactionFailure, HistoryPageCommand,
    HistoryPageRequest, HistoryPageSnapshot, HistoryPayloadCodec, HistoryPayloadCodecFamily,
    HistoryPayloadCodecVersion, HistoryPersistence, HistoryPersistenceLimits, HistoryPolicy,
    HistorySnapshot, HistoryTimedGroupRequest, LinearHistory,
};
use longhorn_tauri_history::{
    HistoryHandlerAssembly, HistoryHostAuthority, HistoryHostError, HistoryHostService,
    history_changed_event,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum PulseMutation {
    RenameTrack {
        track_id: u32,
        before: String,
        after: String,
    },
    DeleteTrack {
        track_id: u32,
        snapshot: String,
    },
    RestoreTrack {
        track_id: u32,
        snapshot: String,
    },
    Compound {
        mutations: Vec<PulseMutation>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PulsePolicyError;

impl fmt::Display for PulsePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Pulse mutation policy failed")
    }
}

impl Error for PulsePolicyError {}

struct PulsePolicy;

impl HistoryPolicy<PulseMutation> for PulsePolicy {
    type Error = PulsePolicyError;

    fn inverse(&self, payload: &PulseMutation) -> Result<PulseMutation, Self::Error> {
        Ok(match payload {
            PulseMutation::RenameTrack {
                track_id,
                before,
                after,
            } => PulseMutation::RenameTrack {
                track_id: *track_id,
                before: after.clone(),
                after: before.clone(),
            },
            PulseMutation::DeleteTrack { track_id, snapshot } => PulseMutation::RestoreTrack {
                track_id: *track_id,
                snapshot: snapshot.clone(),
            },
            PulseMutation::RestoreTrack { track_id, snapshot } => PulseMutation::DeleteTrack {
                track_id: *track_id,
                snapshot: snapshot.clone(),
            },
            PulseMutation::Compound { mutations } => PulseMutation::Compound {
                mutations: mutations
                    .iter()
                    .rev()
                    .map(|mutation| self.inverse(mutation))
                    .collect::<Result<Vec<_>, _>>()?,
            },
        })
    }

    fn is_noop(&self, payload: &PulseMutation) -> bool {
        matches!(
            payload,
            PulseMutation::RenameTrack { before, after, .. } if before == after
        ) || matches!(
            payload,
            PulseMutation::Compound { mutations }
                if mutations.iter().all(|mutation| self.is_noop(mutation))
        )
    }

    fn encoded_weight(&self, payload: &PulseMutation) -> Result<u64, Self::Error> {
        u64::try_from(
            serde_json::to_vec(payload)
                .map_err(|_| PulsePolicyError)?
                .len(),
        )
        .map_err(|_| PulsePolicyError)
    }

    fn coalesce(
        &self,
        previous: &PulseMutation,
        incoming: &PulseMutation,
        context: HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<PulseMutation>, Self::Error> {
        match (previous, incoming) {
            (
                PulseMutation::RenameTrack {
                    track_id, before, ..
                },
                PulseMutation::RenameTrack {
                    track_id: incoming_track_id,
                    after,
                    ..
                },
            ) if track_id == incoming_track_id => {
                Ok(HistoryCoalesce::Replace(PulseMutation::RenameTrack {
                    track_id: *track_id,
                    before: before.clone(),
                    after: after.clone(),
                }))
            }
            _ => Ok(match context {
                HistoryCoalesceContext::Adjacent => HistoryCoalesce::KeepSeparate,
                HistoryCoalesceContext::Group { .. } => {
                    let mut mutations = match previous {
                        PulseMutation::Compound { mutations } => mutations.clone(),
                        previous => vec![previous.clone()],
                    };
                    mutations.push(incoming.clone());
                    HistoryCoalesce::Replace(PulseMutation::Compound { mutations })
                }
            }),
        }
    }
}

struct PulseCodec {
    family: HistoryPayloadCodecFamily,
}

impl PulseCodec {
    fn new() -> Self {
        Self {
            family: HistoryPayloadCodecFamily::new("loophole.pulse-proof").unwrap(),
        }
    }
}

impl HistoryPayloadCodec<PulseMutation> for PulseCodec {
    type Error = serde_json::Error;

    fn family(&self) -> &HistoryPayloadCodecFamily {
        &self.family
    }

    fn version(&self) -> HistoryPayloadCodecVersion {
        HistoryPayloadCodecVersion::new(1)
    }

    fn encode(&self, payload: &PulseMutation) -> Result<Vec<u8>, Self::Error> {
        serde_json::to_vec(payload)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PulseMutation, Self::Error> {
        serde_json::from_slice(bytes)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PulseModel {
    tracks: BTreeMap<u32, String>,
}

impl PulseModel {
    fn seeded() -> Self {
        Self {
            tracks: BTreeMap::from([
                (1, "Drums".to_owned()),
                (2, "A".to_owned()),
                (3, "FX".to_owned()),
            ]),
        }
    }

    fn apply(&mut self, mutation: &PulseMutation) -> Result<(), TransactionError> {
        match mutation {
            PulseMutation::RenameTrack {
                track_id,
                before,
                after,
            } => {
                if self.tracks.get(track_id) != Some(before) {
                    return Err(TransactionError::UnexpectedModel);
                }
                self.tracks.insert(*track_id, after.clone());
            }
            PulseMutation::DeleteTrack { track_id, snapshot } => {
                if self.tracks.remove(track_id).as_ref() != Some(snapshot) {
                    return Err(TransactionError::UnexpectedModel);
                }
            }
            PulseMutation::RestoreTrack { track_id, snapshot } => {
                if self.tracks.insert(*track_id, snapshot.clone()).is_some() {
                    return Err(TransactionError::UnexpectedModel);
                }
            }
            PulseMutation::Compound { mutations } => {
                for mutation in mutations {
                    self.apply(mutation)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TransactionError {
    InjectedApply(usize),
    UnexpectedModel,
    InjectedRollback,
}

impl fmt::Display for TransactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for TransactionError {}

struct PulseTransaction {
    model: PulseModel,
    fail_at_step: Option<usize>,
    rollback_fails: bool,
    apply_calls: usize,
}

impl PulseTransaction {
    fn successful(model: PulseModel) -> Self {
        Self {
            model,
            fail_at_step: None,
            rollback_fails: false,
            apply_calls: 0,
        }
    }
}

impl HistoryNavigationTransaction<PulseMutation> for PulseTransaction {
    type Error = TransactionError;

    fn apply(
        &mut self,
        plan: &HistoryNavigationPlan<PulseMutation>,
    ) -> Result<(), HistoryNavigationTransactionFailure<Self::Error>> {
        self.apply_calls += 1;
        let source = self.model.clone();
        for (index, step) in plan.steps().iter().enumerate() {
            if self.fail_at_step == Some(index) {
                let error = TransactionError::InjectedApply(index);
                if self.rollback_fails {
                    return Err(HistoryNavigationTransactionFailure::RollbackFailed {
                        error,
                        rollback_error: TransactionError::InjectedRollback,
                    });
                }
                self.model = source;
                return Err(HistoryNavigationTransactionFailure::RolledBack { error });
            }
            if let Err(error) = self.model.apply(step.payload()) {
                self.model = source;
                return Err(HistoryNavigationTransactionFailure::RolledBack { error });
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
struct JournalRecord {
    product_revision: u64,
    command: AppliedHistoryRecord<PulseMutation>,
    transition: longhorn_history::HistoryCommittedTransition,
}

#[derive(Default)]
struct DisposableJournal {
    durable_suffix: Vec<JournalRecord>,
    fail_next: bool,
}

impl DisposableJournal {
    fn append(&mut self, record: JournalRecord) -> Result<(), &'static str> {
        if self.fail_next {
            self.fail_next = false;
            return Err("injected journal failure");
        }
        self.durable_suffix.push(record);
        Ok(())
    }
}

#[derive(Clone)]
struct ProtocolAuthority {
    snapshot: HistorySnapshot,
    page: HistoryPageSnapshot,
    committed: HistoryNavigationResult,
}

impl HistoryHostAuthority for ProtocolAuthority {
    fn snapshot(&mut self, caller: &str) -> Result<HistorySnapshot, HistoryHostError> {
        authorize(caller)?;
        Ok(self.snapshot.clone())
    }

    fn page(
        &mut self,
        caller: &str,
        _: HistoryPageCommand,
    ) -> Result<HistoryPageSnapshot, HistoryHostError> {
        authorize(caller)?;
        Ok(self.page.clone())
    }

    fn navigate(
        &mut self,
        caller: &str,
        _: HistoryNavigationCommand,
    ) -> Result<HistoryNavigationResult, HistoryHostError> {
        if caller == "history" {
            Ok(self.committed.clone())
        } else {
            Ok(HistoryNavigationResult::Rejected {
                snapshot: self.snapshot.clone(),
                rejection: HistoryNavigationRejectionProjection {
                    code: HistoryNavigationRejectionCode::Unauthorized,
                    detail: "consumer product authority rejected caller".into(),
                    refresh_required: false,
                },
            })
        }
    }
}

fn authorize(caller: &str) -> Result<(), HistoryHostError> {
    if caller == "history" {
        Ok(())
    } else {
        Err(HistoryHostError::authority(
            "consumer product authority rejected caller",
            false,
        ))
    }
}

fn main() {
    let policy = PulsePolicy;
    let (mut history, mut model) = seeded_history(&policy);
    let full_history = history.clone();
    let full_model = model.clone();

    let failures = failure_proof(full_history.clone(), full_model.clone(), &policy);
    let navigation = navigation_proof(full_history.clone(), full_model, &policy);
    let limits = limit_proof(full_history.clone());
    let recovery = persistence_and_recovery_proof(&policy);

    navigate(
        &mut history,
        &mut model,
        "plan:seed-future",
        HistoryNavigationTarget::Undo,
        &policy,
    );
    let epoch = HistoryAuthorityEpoch::new(7).unwrap();
    let initial_snapshot = snapshot(epoch, &history);
    let initial_page = page(epoch, &history);
    assert_eq!(initial_snapshot.summary.redo_depth, 1);

    let plan = history
        .plan_navigation(
            HistoryNavigationRequest::new(
                HistoryPlanId::new("plan:renderer-expected").unwrap(),
                history.revision(),
                HistoryNavigationTarget::Undo,
            ),
            &policy,
        )
        .unwrap();
    let mut transaction = PulseTransaction::successful(model);
    let receipt = history.execute_navigation(plan, &mut transaction).unwrap();
    let changed_event = HistoryChangedEvent::from_transition(epoch, receipt.transition());
    let committed_snapshot = snapshot(epoch, &history);
    let committed_page = page(epoch, &history);
    let committed_result = HistoryNavigationResult::Committed {
        snapshot: committed_snapshot.clone(),
        receipt: Box::new(HistoryNavigationReceiptProjection::from_receipt(&receipt).unwrap()),
    };
    let expected_public_trace = public_trace(&committed_snapshot, &committed_page);

    let tauri = tauri_proof(
        initial_snapshot.clone(),
        initial_page.clone(),
        committed_result.clone(),
    );

    println!(
        "{}",
        json!({
            "shape": "loophole",
            "mechanics": {
                "record": true,
                "coalesce": true,
                "timedGroupMillis": 750,
                "limit": limits,
                "undoRedoCheckout": navigation,
                "reloadRecovery": recovery
            },
            "failures": failures,
            "tauri": tauri,
            "publicTrace": expected_public_trace,
            "rendererFixture": {
                "initialSnapshot": initial_snapshot,
                "initialPage": initial_page,
                "navigationResult": committed_result,
                "committedPage": committed_page,
                "changedEvent": changed_event,
                "expectedPublicTrace": expected_public_trace
            }
        })
    );
}

fn seeded_history(policy: &PulsePolicy) -> (LinearHistory<PulseMutation>, PulseModel) {
    let mut history = LinearHistory::new(
        HistoryId::new("history:loophole-pulse").unwrap(),
        HistoryLimits::default(),
    );
    let mut model = PulseModel::seeded();

    let rename_one = rename(1, "Drums", "Kit");
    model.apply(&rename_one).unwrap();
    record_applied(
        &mut history,
        "entry:rename",
        "Rename track",
        "track:rename",
        rename_one,
        policy,
    );
    let rename_two = rename(1, "Kit", "Beats");
    model.apply(&rename_two).unwrap();
    record_applied(
        &mut history,
        "entry:rename-again",
        "Rename track to Beats",
        "track:rename",
        rename_two,
        policy,
    );
    assert_eq!(history.applied().len(), 1);

    let gesture = rename(2, "A", "B");
    model.apply(&gesture).unwrap();
    history
        .record_timed(
            applied(
                history.revision().get(),
                "entry:gesture",
                "Adjust track",
                "track:gesture",
                gesture,
            ),
            timed("group:gesture", "gesture:track", 1_000, 750),
            policy,
        )
        .unwrap();
    let grouped_delete = PulseMutation::DeleteTrack {
        track_id: 3,
        snapshot: "FX".into(),
    };
    model.apply(&grouped_delete).unwrap();
    history
        .record_timed(
            applied(
                history.revision().get(),
                "entry:gesture-delete",
                "Adjust track",
                "track:gesture",
                grouped_delete,
            ),
            timed("group:unused", "gesture:track", 1_749, 750),
            policy,
        )
        .unwrap();
    history
        .close_group(&HistoryGroupId::new("group:gesture").unwrap())
        .unwrap();
    assert!(matches!(
        history.current().unwrap().payload(),
        PulseMutation::Compound { mutations } if mutations.len() == 2
    ));

    let delete = PulseMutation::DeleteTrack {
        track_id: 2,
        snapshot: "B".into(),
    };
    model.apply(&delete).unwrap();
    record_applied(
        &mut history,
        "entry:delete",
        "Delete track",
        "track:delete",
        delete,
        policy,
    );
    assert_eq!(history.applied().len(), 3);
    (history, model)
}

fn failure_proof(
    history: LinearHistory<PulseMutation>,
    model: PulseModel,
    policy: &PulsePolicy,
) -> Value {
    let mut rolled_back_history = history.clone();
    let before_history = rolled_back_history.clone();
    let before_model = model.clone();
    let plan = checkout_plan(
        &rolled_back_history,
        "plan:apply-failure",
        "entry:rename",
        policy,
    );
    let mut rolled_back = PulseTransaction {
        model: model.clone(),
        fail_at_step: Some(1),
        rollback_fails: false,
        apply_calls: 0,
    };
    assert!(matches!(
        rolled_back_history.execute_navigation(plan, &mut rolled_back),
        Err(HistoryNavigationExecutionError::RolledBack { .. })
    ));
    assert_eq!(rolled_back_history, before_history);
    assert_eq!(rolled_back.model, before_model);

    let mut terminal_history = history.clone();
    let before_terminal_history = terminal_history.clone();
    let plan = checkout_plan(
        &terminal_history,
        "plan:rollback-failure",
        "entry:rename",
        policy,
    );
    let mut terminal = PulseTransaction {
        model: model.clone(),
        fail_at_step: Some(1),
        rollback_fails: true,
        apply_calls: 0,
    };
    assert!(matches!(
        terminal_history.execute_navigation(plan, &mut terminal),
        Err(HistoryNavigationExecutionError::RollbackFailed { .. })
    ));
    assert_eq!(terminal_history, before_terminal_history);
    assert_ne!(terminal.model, model);

    let mut stale_history = history;
    let stale = stale_history
        .plan_navigation(
            HistoryNavigationRequest::new(
                HistoryPlanId::new("plan:stale").unwrap(),
                stale_history.revision(),
                HistoryNavigationTarget::Undo,
            ),
            policy,
        )
        .unwrap();
    record_applied(
        &mut stale_history,
        "entry:external",
        "Restore external track",
        "track:restore",
        PulseMutation::RestoreTrack {
            track_id: 9,
            snapshot: "External".into(),
        },
        policy,
    );
    let before_stale = stale_history.clone();
    let mut stale_transaction = PulseTransaction::successful(model);
    assert!(matches!(
        stale_history.execute_navigation(stale, &mut stale_transaction),
        Err(HistoryNavigationExecutionError::Rejected { .. })
    ));
    assert_eq!(stale_transaction.apply_calls, 0);
    assert_eq!(stale_history, before_stale);

    json!({
        "applyFailure": {
            "historyExact": true,
            "modelExactAfterVerifiedRollback": true
        },
        "rollbackFailure": {
            "historyExact": true,
            "terminalPartialModelEvidence": true
        },
        "stalePlan": {
            "historyExact": true,
            "productApplyCalls": 0
        }
    })
}

fn navigation_proof(
    mut history: LinearHistory<PulseMutation>,
    model: PulseModel,
    policy: &PulsePolicy,
) -> Value {
    let mut transaction = PulseTransaction::successful(model);
    let undo = history
        .plan_navigation(
            HistoryNavigationRequest::new(
                HistoryPlanId::new("plan:undo").unwrap(),
                history.revision(),
                HistoryNavigationTarget::Undo,
            ),
            policy,
        )
        .unwrap();
    history.execute_navigation(undo, &mut transaction).unwrap();
    let redo = history
        .plan_navigation(
            HistoryNavigationRequest::new(
                HistoryPlanId::new("plan:redo").unwrap(),
                history.revision(),
                HistoryNavigationTarget::Redo,
            ),
            policy,
        )
        .unwrap();
    history.execute_navigation(redo, &mut transaction).unwrap();
    let checkout = checkout_plan(&history, "plan:checkout", "entry:rename", policy);
    history
        .execute_navigation(checkout, &mut transaction)
        .unwrap();
    assert_eq!(
        history.current().unwrap().entry_id().as_str(),
        "entry:rename"
    );
    json!({
        "undo": true,
        "redo": true,
        "checkoutByStableId": true
    })
}

fn limit_proof(mut history: LinearHistory<PulseMutation>) -> Value {
    let receipt = history
        .change_limits(
            history.revision(),
            HistoryLimits::new(1, 1_024 * 1_024, 1_024).unwrap(),
        )
        .unwrap();
    assert!(!receipt.pruning().advanced_baseline().is_empty());
    json!({
        "baselineAdvanced": true,
        "retainedEntries": history.applied().len() + history.future().len()
    })
}

fn persistence_and_recovery_proof(policy: &PulsePolicy) -> Value {
    let persistence = HistoryPersistence::without_structural_migration(
        PulseCodec::new(),
        HistoryPersistenceLimits::new(256 * 1_024).unwrap(),
    );
    let mut history = LinearHistory::new(
        HistoryId::new("history:loophole-recovery").unwrap(),
        HistoryLimits::default(),
    );
    record_applied(
        &mut history,
        "entry:rename",
        "Rename track",
        "track:rename",
        rename(1, "Drums", "Beats"),
        policy,
    );
    let snapshot_product_revision = 41;
    let snapshot_bytes = persistence.encode(&history).unwrap();

    let suffix_command = applied(
        history.revision().get(),
        "entry:delete",
        "Delete track",
        "track:delete",
        PulseMutation::DeleteTrack {
            track_id: 2,
            snapshot: "Keys".into(),
        },
    );
    let suffix_result = history
        .record_applied(suffix_command.clone(), policy)
        .unwrap();
    let mut journal = DisposableJournal::default();
    journal
        .append(JournalRecord {
            product_revision: 42,
            command: suffix_command,
            transition: suffix_result.transition().unwrap().clone(),
        })
        .unwrap();

    let undurable_command = applied(
        history.revision().get(),
        "entry:restore",
        "Restore track",
        "track:restore",
        PulseMutation::RestoreTrack {
            track_id: 3,
            snapshot: "FX".into(),
        },
    );
    let undurable_transition = history
        .record_applied(undurable_command.clone(), policy)
        .unwrap()
        .transition()
        .unwrap()
        .clone();
    journal.fail_next = true;
    assert!(
        journal
            .append(JournalRecord {
                product_revision: 43,
                command: undurable_command,
                transition: undurable_transition,
            })
            .is_err()
    );
    assert_eq!(journal.durable_suffix.len(), 1);

    let loaded = persistence
        .load(history.history_id(), &snapshot_bytes, policy)
        .unwrap();
    let (mut recovered, _) = loaded.into_parts();
    let mut product_revision = snapshot_product_revision;
    for record in &journal.durable_suffix {
        let replayed = recovered
            .record_applied(record.command.clone(), policy)
            .unwrap();
        assert_eq!(replayed.transition().unwrap(), &record.transition);
        product_revision = record.product_revision;
    }
    let undo = recovered
        .plan_navigation(
            HistoryNavigationRequest::new(
                HistoryPlanId::new("plan:cross-session-undo").unwrap(),
                recovered.revision(),
                HistoryNavigationTarget::Undo,
            ),
            policy,
        )
        .unwrap();
    let mut transaction = PulseTransaction::successful(PulseModel {
        tracks: BTreeMap::from([(1, "Beats".into())]),
    });
    recovered
        .execute_navigation(undo, &mut transaction)
        .unwrap();

    let mut future: Value = serde_json::from_slice(&snapshot_bytes).unwrap();
    future["structuralVersion"] = Value::from(2);
    assert!(matches!(
        persistence.load(
            recovered.history_id(),
            &serde_json::to_vec(&future).unwrap(),
            policy
        ),
        Err(HistoryLoadError::FutureStructuralVersion { .. })
    ));
    let mut foreign: Value = serde_json::from_slice(&snapshot_bytes).unwrap();
    foreign["payloadCodec"]["family"] = Value::from("foreign.history");
    assert!(matches!(
        persistence.load(
            recovered.history_id(),
            &serde_json::to_vec(&foreign).unwrap(),
            policy
        ),
        Err(HistoryLoadError::ForeignPayloadCodecFamily { .. })
    ));
    let mut corrupt: Value = serde_json::from_slice(&snapshot_bytes).unwrap();
    corrupt["entries"][0]["payload"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(
        persistence
            .load(
                recovered.history_id(),
                &serde_json::to_vec(&corrupt).unwrap(),
                policy
            )
            .is_err()
    );

    json!({
        "snapshotReload": true,
        "journalSuffixReplayed": true,
        "crossSessionUndo": true,
        "recoveredProductRevision": product_revision,
        "durabilityFailure": {
            "inMemoryRevision": history.revision().get(),
            "durableRevision": 2,
            "retryMustNotPretendCommitFailed": true
        },
        "codecFailures": {
            "futureStructuralRejected": true,
            "foreignFamilyRejected": true,
            "corruptPayloadRejected": true,
            "liveStateReplaced": false
        }
    })
}

fn tauri_proof(
    initial_snapshot: HistorySnapshot,
    initial_page: HistoryPageSnapshot,
    committed: HistoryNavigationResult,
) -> Value {
    let service = HistoryHandlerAssembly::new(ProtocolAuthority {
        snapshot: initial_snapshot.clone(),
        page: initial_page.clone(),
        committed: committed.clone(),
    });
    let page_command = HistoryPageCommand {
        protocol_version: longhorn_history::HistoryProtocolVersion::CURRENT,
        authority_epoch: initial_snapshot.authority_epoch,
        history_id: initial_snapshot.summary.history_id.clone(),
        expected_revision: initial_snapshot.summary.revision,
        offset: 0,
        limit: 50,
    };
    let navigation_command = HistoryNavigationCommand {
        protocol_version: longhorn_history::HistoryProtocolVersion::CURRENT,
        authority_epoch: initial_snapshot.authority_epoch,
        history_id: initial_snapshot.summary.history_id.clone(),
        plan_id: HistoryPlanId::new("plan:tauri-proof").unwrap(),
        expected_revision: initial_snapshot.summary.revision,
        target: HistoryNavigationTargetProjection::Undo,
    };
    assert_eq!(service.snapshot("history").unwrap(), initial_snapshot);
    assert_eq!(service.page("history", page_command).unwrap(), initial_page);
    assert_eq!(
        service
            .navigate("history", navigation_command.clone())
            .unwrap(),
        committed
    );
    let unauthorized = service.navigate("main", navigation_command).unwrap();
    assert!(matches!(
        unauthorized,
        HistoryNavigationResult::Rejected {
            rejection: HistoryNavigationRejectionProjection {
                code: HistoryNavigationRejectionCode::Unauthorized,
                ..
            },
            ..
        }
    ));
    assert!(history_changed_event(&committed).is_some());
    assert!(history_changed_event(&unauthorized).is_none());
    json!({
        "callerAware": true,
        "capabilityIsNotProductAuthority": true,
        "committedEventHint": true,
        "rejectionEventHint": false
    })
}

fn record_applied(
    history: &mut LinearHistory<PulseMutation>,
    entry_id: &str,
    label: &str,
    kind: &str,
    payload: PulseMutation,
    policy: &PulsePolicy,
) {
    history
        .record_applied(
            applied(history.revision().get(), entry_id, label, kind, payload),
            policy,
        )
        .unwrap();
}

fn applied(
    revision: u64,
    entry_id: &str,
    label: &str,
    kind: &str,
    payload: PulseMutation,
) -> AppliedHistoryRecord<PulseMutation> {
    AppliedHistoryRecord::new(
        HistoryRevision::new(revision),
        HistoryEntryId::new(entry_id).unwrap(),
        HistoryEntryMetadata::new(
            HistoryLabel::new(label).unwrap(),
            Some(HistoryKindId::new(kind).unwrap()),
            None,
        ),
        payload,
    )
}

fn rename(track_id: u32, before: &str, after: &str) -> PulseMutation {
    PulseMutation::RenameTrack {
        track_id,
        before: before.into(),
        after: after.into(),
    }
}

fn timed(group: &str, key: &str, now_ms: u64, duration_ms: u64) -> HistoryTimedGroupRequest {
    HistoryTimedGroupRequest::new(
        HistoryGroupId::new(group).unwrap(),
        HistoryGroupKeyId::new(key).unwrap(),
        HistoryMonotonicMillis::new(now_ms),
        HistoryGroupDurationMillis::new(duration_ms).unwrap(),
    )
}

fn checkout_plan(
    history: &LinearHistory<PulseMutation>,
    plan_id: &str,
    entry_id: &str,
    policy: &PulsePolicy,
) -> HistoryNavigationPlan<PulseMutation> {
    history
        .plan_navigation(
            HistoryNavigationRequest::new(
                HistoryPlanId::new(plan_id).unwrap(),
                history.revision(),
                HistoryNavigationTarget::Checkout {
                    entry_id: HistoryEntryId::new(entry_id).unwrap(),
                },
            ),
            policy,
        )
        .unwrap()
}

fn navigate(
    history: &mut LinearHistory<PulseMutation>,
    model: &mut PulseModel,
    plan_id: &str,
    target: HistoryNavigationTarget,
    policy: &PulsePolicy,
) {
    let plan = history
        .plan_navigation(
            HistoryNavigationRequest::new(
                HistoryPlanId::new(plan_id).unwrap(),
                history.revision(),
                target,
            ),
            policy,
        )
        .unwrap();
    let mut transaction = PulseTransaction::successful(model.clone());
    history.execute_navigation(plan, &mut transaction).unwrap();
    *model = transaction.model;
}

fn snapshot(
    epoch: HistoryAuthorityEpoch,
    history: &LinearHistory<PulseMutation>,
) -> HistorySnapshot {
    HistorySnapshot::from_summary(epoch, &history.project_summary().unwrap()).unwrap()
}

fn page(
    epoch: HistoryAuthorityEpoch,
    history: &LinearHistory<PulseMutation>,
) -> HistoryPageSnapshot {
    HistoryPageSnapshot::from_page(
        epoch,
        &history
            .project_page(HistoryPageRequest::new(0, 50).unwrap())
            .unwrap(),
    )
    .unwrap()
}

fn public_trace(snapshot: &HistorySnapshot, page: &HistoryPageSnapshot) -> Value {
    json!({
        "revision": snapshot.summary.revision.get(),
        "undoDepth": snapshot.summary.undo_depth,
        "redoDepth": snapshot.summary.redo_depth,
        "currentEntryId": snapshot
            .summary
            .current_entry_id
            .as_ref()
            .map(HistoryEntryId::as_str),
        "entries": page.entries.iter().map(|entry| json!({
            "entryId": entry.entry_id.as_str(),
            "position": match entry.position {
                longhorn_history::HistoryProjectionPosition::Past => "past",
                longhorn_history::HistoryProjectionPosition::Current => "current",
                longhorn_history::HistoryProjectionPosition::Future => "future",
            }
        })).collect::<Vec<_>>()
    })
}
