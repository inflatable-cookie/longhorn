use std::{fs, path::Path, time::Duration};

use longhorn_core::{DomainId, SchemaVersion};
use serde_json::Value;
use tempfile::TempDir;

use crate::{
    BackupApplication, BackupKind, BackupMetadata, BackupProducer, ConfigDomain,
    CoordinationAuthority, DomainDescriptor, DomainFilePath, DomainIssue, DomainLocation,
    DurabilityRequirement, LoadOutcome, MigrationStep, MutationError, MutationOptions,
    RestoreCurrentEvidence, RestoreOperationState, RestoreRecoveryOutcome, Sha256Digest,
    StorageClass, StorageRoots, UnavailableState,
};

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
    _temp: TempDir,
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
            _temp: temp,
            target: file.full_path().to_path_buf(),
            store,
            domain,
        }
    }

    fn journal(&self, phase: JournalPhase, old: &[u8], new: &[u8]) {
        fs::write(&self.target, old).unwrap();
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
        let authority = self.store.coordinator.authority_root();
        let mut state = journal::persist_rollback(
            authority,
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
                operation_id: "test-operation".into(),
                plan_digest: Sha256Digest::from_bytes(b"plan"),
                safety_path: Path::new("/tmp/safety.longhorn-backup").into(),
                safety_sha256: Sha256Digest::from_bytes(b"safety"),
            },
        )
        .unwrap();
        state.phase = phase;
        journal::publish(authority, &state).unwrap();
        fs::write(&self.target, new).unwrap();
    }
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
