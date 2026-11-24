use std::{fs, io, time::Duration};

use longhorn_core::{DomainId, SchemaVersion};
use serde_json::Value;
use tempfile::TempDir;

use crate::StorageRootsResolve;
use crate::{
    BackupApplication, BackupCaptureError, BackupCaptureOptions, BackupKind, BackupLimits,
    BackupMetadata, BackupProducer, ConfigDomain, CoordinationAuthority, DomainDescriptor,
    DomainFilePath, DomainIssue, DomainLocation, DurabilityRequirement, LoadOutcome, MigrationStep,
    MutationError, MutationOptions, RestoreCurrentEvidence, RestoreOperationState,
    RestoreRecoveryOptions, RestoreRecoveryOutcome, Sha256Digest, StorageClass, UnavailableState,
};
use longhorn_storage_layout::StorageRoots;

use super::*;
use crate::backup::restore::{
    journal::{JournalDescriptor, JournalSeed, RollbackEntry},
    types::StagedDomain,
};

struct TestDomain {
    descriptor: DomainDescriptor,
}

impl ConfigDomain for TestDomain {
    type Value = Value;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        serde_json::json!({})
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        Ok(value)
    }

    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        Ok(value.clone())
    }

    fn validate(&self, _value: &Self::Value) -> Result<(), DomainIssue> {
        Ok(())
    }

    fn validate_raw(
        &self,
        _schema_version: SchemaVersion,
        _value: &Value,
    ) -> Result<(), DomainIssue> {
        Ok(())
    }

    fn migrate_one(
        &self,
        _from: SchemaVersion,
        _value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        Ok(None)
    }
}

struct Fixture {
    temp: TempDir,
    store: ConfigStore,
    domain: TestDomain,
    target: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let paths = [
            "config", "data", "state", "cache", "runtime", "log", "backups",
        ]
        .map(|name| temp.path().join(name));
        for path in &paths {
            fs::create_dir_all(path).unwrap();
        }
        let roots = StorageRoots::new(
            &paths[0], &paths[1], &paths[2], &paths[3], &paths[4], &paths[5], &paths[6],
        )
        .unwrap();
        let coordination = CoordinationAuthority::new(&paths[1]).unwrap();
        let mut store = ConfigStore::new(roots, coordination);
        let domain = TestDomain {
            descriptor: DomainDescriptor::new(
                DomainId::new("test.restore").unwrap(),
                SchemaVersion::new(1).unwrap(),
                StorageClass::UserConfig,
                Some(DomainFilePath::new("restore/state.json").unwrap()),
            )
            .unwrap(),
        };
        store.register(&domain).unwrap();
        let DomainLocation::File(file) = store.roots.resolve(domain.descriptor()) else {
            unreachable!()
        };
        fs::create_dir_all(file.full_path().parent().unwrap()).unwrap();
        Self {
            temp,
            target: file.full_path().to_path_buf(),
            store,
            domain,
        }
    }

    fn journal(&self, phase: JournalPhase, old: &[u8], new: &[u8]) {
        fs::write(&self.target, old).unwrap();
        let mut state = self
            .prepare_journal("test-operation".into(), old, new)
            .unwrap();
        state.phase = phase;
        journal::publish(self.store.coordinator.authority_root(), &state).unwrap();
        fs::write(&self.target, new).unwrap();
    }

    fn prepare_journal(
        &self,
        operation_id: String,
        old: &[u8],
        new: &[u8],
    ) -> io::Result<RestoreJournal> {
        let staged = StagedDomain {
            domain: self.domain.descriptor.id().clone(),
            action: RestoreAction::Replace,
            path: self.target.clone(),
            current: RestoreCurrentEvidence::Present {
                byte_length: old.len() as u64,
                sha256: Sha256Digest::from_bytes(old),
            },
            schema_version: Some(SchemaVersion::new(1).unwrap()),
            bytes: Some(new.to_vec()),
        };
        journal::persist_rollback(
            self.store.coordinator.authority_root(),
            std::slice::from_ref(&staged),
            &[RollbackEntry {
                domain: staged.domain.clone(),
                bytes: Some(old.to_vec()),
            }],
            &[JournalDescriptor {
                domain: staged.domain.clone(),
                storage_class: StorageClass::UserConfig,
                relative_path: "restore/state.json".into(),
            }],
            JournalSeed {
                operation_id,
                plan_digest: Sha256Digest::from_bytes(b"plan"),
                safety_path: self.temp.path().join("safety.longhorn-backup"),
                safety_sha256: Sha256Digest::from_bytes(b"safety"),
            },
        )
    }
}

const MAX_JOURNAL_BYTES: usize = 4 * 1024 * 1024;

fn operation_id_at_limit(fixture: &Fixture, phase: JournalPhase, old: &[u8], new: &[u8]) -> String {
    let mut state = fixture.prepare_journal(String::new(), old, new).unwrap();
    state.phase = phase;
    let overhead = serde_json::to_vec(&state).unwrap().len();
    "x".repeat(MAX_JOURNAL_BYTES - overhead)
}

#[test]
fn capture_refuses_active_and_recovery_required_restore_journals() {
    for (phase, expected) in [
        (JournalPhase::Applying, RestoreOperationState::Active),
        (JournalPhase::Succeeded, RestoreOperationState::Active),
        (
            JournalPhase::RecoveryRequired,
            RestoreOperationState::RecoveryRequired,
        ),
    ] {
        let sample = Fixture::new();
        sample.journal(phase, b"prior-state", b"partly-restored-state");
        let mut catalog = BackupCatalog::new();
        catalog.include(&sample.domain).unwrap();
        let metadata = BackupMetadata::new(
            "daily",
            BackupKind::Operational,
            "2026-01-01T00:00:00Z",
            BackupApplication::new("com.example.desktop", "1").unwrap(),
            BackupProducer::new("longhorn-config", "1").unwrap(),
        )
        .unwrap();
        let result = sample.store.capture_backup(
            &catalog,
            &BackupScope::AllRegistered,
            metadata,
            BackupCaptureOptions::new(Duration::ZERO, BackupLimits::default()),
        );
        assert!(
            matches!(result, Err(BackupCaptureError::RestoreUnavailable(state)) if state == expected)
        );
        assert_eq!(fs::read(&sample.target).unwrap(), b"partly-restored-state");
        assert_eq!(sample.store.restore_operation_state(), expected);
    }
}

#[test]
fn ordinary_recovery_refuses_oversized_rollback_with_matching_evidence() {
    let sample = Fixture::new();
    let oversized = vec![b'x'; crate::store::load::max_live_domain_bytes() + 1];
    sample.journal(JournalPhase::Applying, &oversized, b"current-state");
    let error = sample
        .store
        .recover_restore(RestoreRecoveryOptions::new(Duration::ZERO))
        .unwrap_err();
    assert!(
        error
            .detail
            .contains("rollback payload exceeds live domain byte limit"),
        "{error}"
    );
    assert_eq!(fs::read(&sample.target).unwrap(), b"current-state");
    assert_eq!(
        sample.store.restore_operation_state(),
        RestoreOperationState::RecoveryRequired
    );
}

#[test]
fn ordinary_restore_rejects_admission_when_only_the_shorter_phase_fits() {
    let fixture = Fixture::new();
    let old = b"old-state";
    let new = b"new-state";
    fs::write(&fixture.target, old).unwrap();
    let operation_id = operation_id_at_limit(&fixture, JournalPhase::Applying, old, new);

    let Err(error) = fixture.prepare_journal(operation_id, old, new) else {
        panic!("restore admitted a journal whose recovery phase exceeds the byte limit");
    };
    assert!(error.to_string().contains("exceeds byte limit"), "{error}");
    assert_eq!(fs::read(&fixture.target).unwrap(), old);
    assert_eq!(
        fixture.store.restore_operation_state(),
        RestoreOperationState::Inactive
    );
}

#[test]
fn interrupted_ordinary_restore_recovers_at_the_journal_limit() {
    let fixture = Fixture::new();
    let old = b"old-state";
    let new = b"new-state";
    let operation_id = operation_id_at_limit(&fixture, JournalPhase::RecoveryRequired, old, new);
    let mut state = fixture.prepare_journal(operation_id, old, new).unwrap();
    let authority = fixture.store.coordinator.authority_root();
    state.phase = JournalPhase::Applying;
    journal::publish(authority, &state).unwrap();
    fs::write(&fixture.target, new).unwrap();

    let rollback = authority.join(".longhorn/restore/rollback/test.restore.rollback");
    fs::write(&rollback, b"wrong-old").unwrap();
    let options = RestoreRecoveryOptions::new(Duration::from_secs(1));
    let error = fixture.store.recover_restore(options).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("rollback payload length or digest"),
        "{error}"
    );
    assert_eq!(
        fixture.store.restore_operation_state(),
        RestoreOperationState::RecoveryRequired
    );
    assert_eq!(fs::read(&fixture.target).unwrap(), new);
    assert_eq!(
        fs::metadata(journal::journal_path(authority))
            .unwrap()
            .len(),
        MAX_JOURNAL_BYTES as u64
    );

    fs::write(rollback, old).unwrap();
    let receipt = fixture.store.recover_restore(options).unwrap();
    assert_eq!(receipt.outcome(), RestoreRecoveryOutcome::RolledBack);
    assert_eq!(fs::read(&fixture.target).unwrap(), old);
    assert_eq!(
        fixture.store.restore_operation_state(),
        RestoreOperationState::Inactive
    );
}

#[test]
fn every_nonterminal_crash_phase_rolls_back_and_recovery_is_idempotent() {
    for phase in [
        JournalPhase::Prepared,
        JournalPhase::Applying,
        JournalPhase::Verifying,
        JournalPhase::RollingBack,
        JournalPhase::RecoveryRequired,
    ] {
        let fixture = Fixture::new();
        fixture.journal(phase, b"old-state", b"new-state");
        assert_eq!(
            fixture.store.restore_operation_state(),
            if phase == JournalPhase::RecoveryRequired {
                RestoreOperationState::RecoveryRequired
            } else {
                RestoreOperationState::Active
            }
        );
        assert!(
            matches!(
                fixture.store.load(&fixture.domain).unwrap(),
                LoadOutcome::Unavailable(UnavailableState::RestoreRecoveryRequired)
                    if phase == JournalPhase::RecoveryRequired
            ) || matches!(
                fixture.store.load(&fixture.domain).unwrap(),
                LoadOutcome::Unavailable(UnavailableState::RestoreActive)
            )
        );
        let receipt = recovery::recover(&fixture.store, Duration::from_secs(1)).unwrap();
        assert_eq!(receipt.outcome(), RestoreRecoveryOutcome::RolledBack);
        assert_eq!(fs::read(&fixture.target).unwrap(), b"old-state");
        let repeated = recovery::recover(&fixture.store, Duration::from_secs(1)).unwrap();
        assert_eq!(repeated.outcome(), RestoreRecoveryOutcome::NoRecoveryNeeded);
    }
}

#[test]
fn verified_success_phase_keeps_target_and_only_cleans_terminal_material() {
    let fixture = Fixture::new();
    fixture.journal(JournalPhase::Succeeded, b"old-state", b"new-state");
    let receipt = recovery::recover(&fixture.store, Duration::from_secs(1)).unwrap();
    assert_eq!(receipt.outcome(), RestoreRecoveryOutcome::TerminalCleanup);
    assert_eq!(fs::read(&fixture.target).unwrap(), b"new-state");
    assert_eq!(
        fixture.store.restore_operation_state(),
        RestoreOperationState::Inactive
    );
}

#[test]
fn verified_rolled_back_phase_keeps_old_state_and_only_cleans_terminal_material() {
    let fixture = Fixture::new();
    fixture.journal(JournalPhase::RolledBack, b"old-state", b"new-state");
    fs::write(&fixture.target, b"old-state").unwrap();
    let receipt = recovery::recover(&fixture.store, Duration::from_secs(1)).unwrap();
    assert_eq!(receipt.outcome(), RestoreRecoveryOutcome::TerminalCleanup);
    assert_eq!(fs::read(&fixture.target).unwrap(), b"old-state");
    assert_eq!(
        fixture.store.restore_operation_state(),
        RestoreOperationState::Inactive
    );
}

#[test]
fn corrupt_rollback_payload_retains_journal_and_requires_recovery() {
    let fixture = Fixture::new();
    fixture.journal(JournalPhase::Applying, b"old-state", b"new-state");
    let rollback = fixture
        .store
        .coordinator
        .authority_root()
        .join(".longhorn/restore/rollback/test.restore.rollback");
    fs::write(rollback, b"corrupt").unwrap();
    assert!(recovery::recover(&fixture.store, Duration::from_secs(1)).is_err());
    assert_eq!(
        fixture.store.restore_operation_state(),
        RestoreOperationState::RecoveryRequired
    );
    assert!(matches!(
        fixture.store.load(&fixture.domain).unwrap(),
        LoadOutcome::Unavailable(UnavailableState::RestoreRecoveryRequired)
    ));
    let mutation = fixture.store.mutate(
        &fixture.domain,
        MutationOptions::new(Duration::from_secs(1), DurabilityRequirement::Durable),
        |_| Ok(()),
    );
    assert!(matches!(mutation, Err(MutationError::RestoreRecovery(_))));
    assert_eq!(fs::read(&fixture.target).unwrap(), b"new-state");
}

#[test]
fn ordinary_post_journal_failure_rolls_back_before_return() {
    let fixture = Fixture::new();
    fixture.journal(JournalPhase::Applying, b"old-state", b"new-state");
    let mut state = journal::load(fixture.store.coordinator.authority_root())
        .unwrap()
        .unwrap();
    let error = rollback_after_failure(
        &fixture.store,
        &mut state,
        RestoreExecutionStage::VerifyTarget,
        Some(fixture.domain.descriptor().id().clone()),
        "injected verification failure",
    );
    assert_eq!(error.terminal, RestoreFailureTerminal::RolledBack);
    assert_eq!(fs::read(&fixture.target).unwrap(), b"old-state");
    assert_eq!(
        fixture.store.restore_operation_state(),
        RestoreOperationState::Inactive
    );
}

#[test]
fn safety_metadata_kind_is_explicit() {
    let metadata = BackupMetadata::new(
        "safety",
        BackupKind::PreRestore,
        "2026-07-28T12:00:00Z",
        BackupApplication::new("test", "1").unwrap(),
        BackupProducer::new("test", "1").unwrap(),
    )
    .unwrap();
    assert_eq!(metadata.kind(), BackupKind::PreRestore);
}

#[test]
fn stale_current_file_during_rollback_capture_reports_recheck_current() {
    let fixture = Fixture::new();
    let initial = br#"{"domain":"test.restore","schemaVersion":1,"value":{}}"#;
    fs::write(&fixture.target, initial).unwrap();
    let target_bytes = br#"{"domain":"test.restore","schemaVersion":1,"value":{"a":1}}"#.to_vec();
    let staging = RestoreStaging {
        archive_sha256: Sha256Digest::from_bytes(b"archive"),
        plan_digest: Sha256Digest::from_bytes(b"plan"),
        receipt: crate::RestoreStagingReceipt {
            selected: 1,
            documents: 1,
            deletions: 0,
            unchanged: 0,
            total_document_bytes: target_bytes.len() as u64,
        },
        domains: vec![StagedDomain {
            domain: fixture.domain.descriptor.id().clone(),
            action: RestoreAction::Replace,
            path: fixture.target.clone(),
            current: RestoreCurrentEvidence::Present {
                byte_length: initial.len() as u64,
                sha256: Sha256Digest::from_bytes(initial),
            },
            schema_version: Some(SchemaVersion::new(1).unwrap()),
            bytes: Some(target_bytes),
        }],
    };
    // Mutate the live file after staging so the staged current evidence is stale.
    fs::write(
        &fixture.target,
        br#"{"domain":"test.restore","schemaVersion":1,"value":{"changed":true}}"#,
    )
    .unwrap();

    let error =
        transaction::capture_rollback(&fixture.store, &staging, crate::BackupLimits::default())
            .err()
            .expect("stale file must be rejected");
    assert_eq!(error.stage, RestoreExecutionStage::RecheckCurrent);
    assert_eq!(error.terminal, RestoreFailureTerminal::NoLiveMutation);
}
