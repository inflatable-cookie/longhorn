use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

use longhorn_config::{
    BackupAdapterGroupedApplyKind, BackupCaptureOptions, RestoreAdapterGroupExecutionOptions,
    RestoreAdapterGroupExecutionStage, RestoreAdapterGroupRecoveryOutcome, RestoreFailureTerminal,
    RestoreOperationState,
};

use super::*;

#[derive(Clone, Copy)]
enum FailureMode {
    None = 0,
    Stage = 1,
    Apply = 2,
    PanicApply = 3,
    VerifyOnce = 4,
    PanicVerify = 5,
    PanicRollback = 6,
}

struct FileAdapter {
    id: BackupAdapterId,
    capabilities: BackupAdapterCapabilities,
    path: PathBuf,
    failure: AtomicU8,
    verify_mismatch_pending: AtomicBool,
}

impl FileAdapter {
    fn new(path: PathBuf, failure: FailureMode) -> Self {
        Self::with_id(path, failure, "grouped-file-v1")
    }

    fn with_id(path: PathBuf, failure: FailureMode, id: &str) -> Self {
        Self::with_participation(
            path,
            failure,
            id,
            BackupAdapterRestoreParticipation::GroupedFailureAtomic,
        )
    }

    fn with_participation(
        path: PathBuf,
        failure: FailureMode,
        id: &str,
        participation: BackupAdapterRestoreParticipation,
    ) -> Self {
        Self {
            id: BackupAdapterId::new(id).unwrap(),
            capabilities: BackupAdapterCapabilities::new(
                BackupAdapterCaptureMode::CoordinatedBounded,
                participation,
            ),
            path,
            failure: AtomicU8::new(failure as u8),
            verify_mismatch_pending: AtomicBool::new(false),
        }
    }

    fn mode(&self) -> u8 {
        self.failure.load(Ordering::SeqCst)
    }

    fn evidence(&self) -> Option<Sha256Digest> {
        self.path
            .is_file()
            .then(|| Sha256Digest::from_bytes(&fs::read(&self.path).unwrap()))
    }
}

impl BackupAdapter for FileAdapter {
    fn id(&self) -> &BackupAdapterId {
        &self.id
    }

    fn capabilities(&self) -> &BackupAdapterCapabilities {
        &self.capabilities
    }

    fn capture(
        &self,
        request: BackupAdapterCaptureRequest<'_>,
    ) -> Result<BackupAdapterCapture, BackupAdapterError> {
        let bytes = fs::read(&self.path).map_err(|_| adapter_failure("group-file-capture"))?;
        if bytes.len() > request.limits().max_domain_bytes() {
            return Err(adapter_failure("group-file-capture-limit"));
        }
        Ok(BackupAdapterCapture::Present {
            source_schema_version: request.descriptor().schema_version(),
            payloads: vec![BackupAdapterPayload::new(
                BackupAdapterRelativePath::new("value.bin").unwrap(),
                bytes,
            )],
        })
    }

    fn inspect(
        &self,
        request: BackupAdapterInspectRequest<'_>,
    ) -> Result<BackupAdapterRestorePreview, BackupAdapterError> {
        let [payload] = request.payloads() else {
            return Err(adapter_failure("group-file-inspect-count"));
        };
        Ok(BackupAdapterRestorePreview::new(
            Sha256Digest::from_bytes(payload.bytes()),
            self.evidence(),
        ))
    }

    fn restore(
        &self,
        _request: BackupAdapterRestoreRequest<'_>,
    ) -> Result<BackupAdapterRestoreOutcome, BackupAdapterError> {
        Err(adapter_failure("group-file-requires-group"))
    }

    fn grouped_restore(&self) -> Option<&dyn BackupAdapterGroupedRestore> {
        Some(self)
    }
}

impl BackupAdapterGroupedRestore for FileAdapter {
    fn stage(
        &self,
        request: BackupAdapterGroupedStageRequest<'_>,
    ) -> Result<BackupAdapterRestoreStage, BackupAdapterError> {
        if self.mode() == FailureMode::Stage as u8 {
            return Err(adapter_failure("group-file-stage-injected"));
        }
        let [payload] = request.inspect().payloads() else {
            return Err(adapter_failure("group-file-stage-count"));
        };
        let rollback = if self.path.is_file() {
            vec![BackupAdapterPayload::new(
                BackupAdapterRelativePath::new("value.bin").unwrap(),
                fs::read(&self.path).map_err(|_| adapter_failure("group-file-stage-old"))?,
            )]
        } else {
            Vec::new()
        };
        Ok(BackupAdapterRestoreStage::new(
            vec![BackupAdapterPayload::new(
                BackupAdapterRelativePath::new("value.bin").unwrap(),
                payload.bytes().to_vec(),
            )],
            rollback,
            request.preview().target_evidence().clone(),
            request.preview().current_evidence().cloned(),
        ))
    }

    fn apply(
        &self,
        request: BackupAdapterGroupedApplyRequest<'_>,
    ) -> Result<(), BackupAdapterError> {
        if request.kind() == BackupAdapterGroupedApplyKind::Target {
            match self.mode() {
                mode if mode == FailureMode::Apply as u8 => {
                    return Err(adapter_failure("group-file-apply-injected"));
                }
                mode if mode == FailureMode::PanicApply as u8 => {
                    panic!("simulated grouped restore process interruption");
                }
                mode if mode == FailureMode::VerifyOnce as u8
                    || mode == FailureMode::PanicRollback as u8 =>
                {
                    self.verify_mismatch_pending.store(true, Ordering::SeqCst);
                }
                _ => {}
            }
        }
        if request.kind() == BackupAdapterGroupedApplyKind::Rollback
            && self.mode() == FailureMode::PanicRollback as u8
        {
            panic!("simulated interruption during grouped rollback");
        }
        if request.expected_evidence().is_none() {
            match fs::remove_file(&self.path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(adapter_failure("group-file-remove")),
            }
            return Ok(());
        }
        let [payload] = request.payloads() else {
            return Err(adapter_failure("group-file-apply-count"));
        };
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|_| adapter_failure("group-file-parent"))?;
        }
        fs::write(&self.path, payload.bytes()).map_err(|_| adapter_failure("group-file-write"))
    }

    fn verify(
        &self,
        _request: BackupAdapterGroupedVerifyRequest<'_>,
    ) -> Result<Option<Sha256Digest>, BackupAdapterError> {
        if self.mode() == FailureMode::PanicVerify as u8 {
            panic!("simulated interruption during grouped verification");
        }
        if self.verify_mismatch_pending.swap(false, Ordering::SeqCst) {
            return Ok(Some(Sha256Digest::from_bytes(b"injected-mismatch")));
        }
        Ok(self.evidence())
    }
}

struct GroupFixture {
    fixture: Fixture,
    first: OpaqueDomain,
    second: OpaqueDomain,
    archive: longhorn_config::BackupArchiveInspection,
    first_path: PathBuf,
    second_path: PathBuf,
}

impl GroupFixture {
    fn new() -> Self {
        let fixture = Fixture::new();
        let first = OpaqueDomain::new(
            "group.first",
            StorageClass::UserConfig,
            "group/first.json",
            &[],
        );
        let second = OpaqueDomain::new(
            "group.second",
            StorageClass::MachineState,
            "group/second.json",
            &[],
        );
        let source_first_path = fixture.temp.path().join("source-first.bin");
        let source_second_path = fixture.temp.path().join("source-second.bin");
        fs::write(&source_first_path, b"new-first").unwrap();
        fs::write(&source_second_path, b"new-second").unwrap();
        let source_first = FileAdapter::new(source_first_path, FailureMode::None);
        let source_second = FileAdapter::new(source_second_path, FailureMode::None);
        let mut source_store = fixture.store();
        source_store.register(&first).unwrap();
        source_store.register(&second).unwrap();
        let mut source_catalog = BackupCatalog::new();
        source_catalog.custom(&first, &source_first).unwrap();
        source_catalog.custom(&second, &source_second).unwrap();
        let snapshot = source_store
            .capture_backup(
                &source_catalog,
                &BackupScope::AllRegistered,
                group_metadata(),
                BackupCaptureOptions::new(Duration::from_secs(2), BackupLimits::default()),
            )
            .unwrap();
        let encoded = encode_backup_archive(&snapshot, BackupArchiveLimits::default()).unwrap();
        let archive =
            inspect_backup_archive(encoded.bytes(), BackupArchiveLimits::default()).unwrap();
        let first_path = fixture.temp.path().join("target-first.bin");
        let second_path = fixture.temp.path().join("target-second.bin");
        fs::write(&first_path, b"old-first").unwrap();
        fs::write(&second_path, b"old-second").unwrap();
        Self {
            fixture,
            first,
            second,
            archive,
            first_path,
            second_path,
        }
    }

    fn store(&self) -> longhorn_config::ConfigStore {
        let mut store = self.fixture.store();
        store.register(&self.first).unwrap();
        store.register(&self.second).unwrap();
        store
    }
}

#[test]
fn group_plan_rejects_empty_duplicate_and_separate_selection() {
    let group = GroupFixture::new();
    let first = FileAdapter::new(group.first_path.clone(), FailureMode::None);
    let second = FileAdapter::new(group.second_path.clone(), FailureMode::None);
    let store = group.store();
    let mut catalog = BackupCatalog::new();
    catalog.custom(&group.first, &first).unwrap();
    catalog.custom(&group.second, &second).unwrap();
    let inspection = inspect(&store, &catalog, &group.archive);

    assert!(matches!(
        store.plan_grouped_adapter_restore(&inspection, Vec::new()),
        Err(longhorn_config::RestoreAdapterGroupPlanError::Empty)
    ));
    assert!(matches!(
        store.plan_grouped_adapter_restore(
            &inspection,
            [
                group.first.descriptor().id().clone(),
                group.first.descriptor().id().clone(),
            ]
        ),
        Err(longhorn_config::RestoreAdapterGroupPlanError::DuplicateDomain { .. })
    ));

    let separate = FileAdapter::with_participation(
        group.first_path.clone(),
        FailureMode::None,
        "grouped-file-v1",
        BackupAdapterRestoreParticipation::Separate,
    );
    let mut separate_catalog = BackupCatalog::new();
    separate_catalog.custom(&group.first, &separate).unwrap();
    separate_catalog.custom(&group.second, &second).unwrap();
    let separate_inspection = inspect(&store, &separate_catalog, &group.archive);
    assert!(matches!(
        store.plan_grouped_adapter_restore(
            &separate_inspection,
            [group.first.descriptor().id().clone()]
        ),
        Err(longhorn_config::RestoreAdapterGroupPlanError::GroupedParticipationRequired { .. })
    ));
}

#[test]
fn group_confirmation_binds_the_complete_selection_before_mutation() {
    let group = GroupFixture::new();
    let first = FileAdapter::new(group.first_path.clone(), FailureMode::None);
    let second = FileAdapter::new(group.second_path.clone(), FailureMode::None);
    let store = group.store();
    let mut catalog = BackupCatalog::new();
    catalog.custom(&group.first, &first).unwrap();
    catalog.custom(&group.second, &second).unwrap();
    let inspection = inspect(&store, &catalog, &group.archive);
    let plan = plan(&store, &inspection, &group);
    let wrong_confirmation = Sha256Digest::from_bytes(b"wrong-group-confirmation");

    let error = store
        .execute_grouped_adapter_restore(
            &catalog,
            &group.archive,
            &inspection,
            &plan,
            &wrong_confirmation,
            group_options(),
        )
        .unwrap_err();

    assert_eq!(
        error.stage(),
        RestoreAdapterGroupExecutionStage::ValidatePlan
    );
    assert_eq!(error.terminal(), RestoreFailureTerminal::NoLiveMutation);
    assert_eq!(fs::read(&group.first_path).unwrap(), b"old-first");
    assert_eq!(fs::read(&group.second_path).unwrap(), b"old-second");
}

#[test]
fn stage_stale_apply_and_verify_failures_never_leave_a_mixed_generation() {
    for (mode, stage, terminal) in [
        (
            FailureMode::Stage,
            RestoreAdapterGroupExecutionStage::Stage,
            RestoreFailureTerminal::NoLiveMutation,
        ),
        (
            FailureMode::Apply,
            RestoreAdapterGroupExecutionStage::ApplyTarget,
            RestoreFailureTerminal::RolledBack,
        ),
        (
            FailureMode::VerifyOnce,
            RestoreAdapterGroupExecutionStage::VerifyTarget,
            RestoreFailureTerminal::RolledBack,
        ),
    ] {
        let group = GroupFixture::new();
        let first = FileAdapter::new(group.first_path.clone(), FailureMode::None);
        let second = FileAdapter::new(group.second_path.clone(), mode);
        let store = group.store();
        let mut catalog = BackupCatalog::new();
        catalog.custom(&group.first, &first).unwrap();
        catalog.custom(&group.second, &second).unwrap();
        let inspection = inspect(&store, &catalog, &group.archive);
        let plan = plan(&store, &inspection, &group);
        let error = store
            .execute_grouped_adapter_restore(
                &catalog,
                &group.archive,
                &inspection,
                &plan,
                plan.confirmation_digest(),
                group_options(),
            )
            .unwrap_err();
        assert_eq!(error.stage(), stage);
        assert_eq!(error.terminal(), terminal);
        assert_eq!(fs::read(&group.first_path).unwrap(), b"old-first");
        assert_eq!(fs::read(&group.second_path).unwrap(), b"old-second");
    }

    let group = GroupFixture::new();
    let first = FileAdapter::new(group.first_path.clone(), FailureMode::None);
    let second = FileAdapter::new(group.second_path.clone(), FailureMode::None);
    let store = group.store();
    let mut catalog = BackupCatalog::new();
    catalog.custom(&group.first, &first).unwrap();
    catalog.custom(&group.second, &second).unwrap();
    let inspection = inspect(&store, &catalog, &group.archive);
    let plan = plan(&store, &inspection, &group);
    fs::write(&group.second_path, b"externally-changed").unwrap();
    let error = store
        .execute_grouped_adapter_restore(
            &catalog,
            &group.archive,
            &inspection,
            &plan,
            plan.confirmation_digest(),
            group_options(),
        )
        .unwrap_err();
    assert_eq!(error.stage(), RestoreAdapterGroupExecutionStage::Reinspect);
    assert_eq!(error.terminal(), RestoreFailureTerminal::NoLiveMutation);
    assert_eq!(fs::read(&group.first_path).unwrap(), b"old-first");
    assert_eq!(fs::read(&group.second_path).unwrap(), b"externally-changed");
}

#[test]
fn process_interruption_phases_block_writes_and_boot_recover_the_complete_group() {
    for mode in [
        FailureMode::PanicApply,
        FailureMode::PanicVerify,
        FailureMode::PanicRollback,
    ] {
        let group = GroupFixture::new();
        let first = FileAdapter::new(group.first_path.clone(), FailureMode::None);
        let second = FileAdapter::new(group.second_path.clone(), mode);
        let store = group.store();
        let mut catalog = BackupCatalog::new();
        catalog.custom(&group.first, &first).unwrap();
        catalog.custom(&group.second, &second).unwrap();
        let inspection = inspect(&store, &catalog, &group.archive);
        let plan = plan(&store, &inspection, &group);
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _ = store.execute_grouped_adapter_restore(
                    &catalog,
                    &group.archive,
                    &inspection,
                    &plan,
                    plan.confirmation_digest(),
                    group_options(),
                );
            }))
            .is_err()
        );
        assert_eq!(
            store.restore_operation_state(),
            RestoreOperationState::Active
        );
        assert!(matches!(
            store.execute_adapter_restore(
                &catalog,
                &group.archive,
                &inspection,
                group.first.descriptor().id(),
                inspection
                    .adapter_confirmation(group.first.descriptor().id())
                    .unwrap(),
                RestoreAdapterRequirement::AllowSeparate,
            ),
            Err(RestoreAdapterError::RestoreUnavailable)
        ));
        assert!(matches!(
            store.load(&group.first).unwrap(),
            longhorn_config::LoadOutcome::Unavailable(_)
        ));

        let recovered_first = FileAdapter::new(group.first_path.clone(), FailureMode::None);
        let recovered_second = FileAdapter::new(group.second_path.clone(), FailureMode::None);
        let recovered_store = group.store();
        let mut recovered_catalog = BackupCatalog::new();
        recovered_catalog
            .custom(&group.first, &recovered_first)
            .unwrap();
        recovered_catalog
            .custom(&group.second, &recovered_second)
            .unwrap();
        let receipt = recovered_store
            .recover_grouped_adapter_restore(&recovered_catalog, Duration::from_secs(2))
            .unwrap();
        assert_eq!(
            receipt.outcome(),
            RestoreAdapterGroupRecoveryOutcome::RolledBack
        );
        assert_eq!(fs::read(&group.first_path).unwrap(), b"old-first");
        assert_eq!(fs::read(&group.second_path).unwrap(), b"old-second");
        assert_eq!(
            recovered_store.restore_operation_state(),
            RestoreOperationState::Inactive
        );
    }
}

#[test]
fn changed_boot_catalog_and_corrupt_journal_fail_closed() {
    let group = GroupFixture::new();
    leave_interrupted_group(&group);
    let first = FileAdapter::new(group.first_path.clone(), FailureMode::None);
    let changed_second = FileAdapter::with_id(
        group.second_path.clone(),
        FailureMode::None,
        "changed-grouped-file-v1",
    );
    let store = group.store();
    let mut catalog = BackupCatalog::new();
    catalog.custom(&group.first, &first).unwrap();
    catalog.custom(&group.second, &changed_second).unwrap();
    assert!(
        store
            .recover_grouped_adapter_restore(&catalog, Duration::from_secs(2))
            .is_err()
    );
    assert_eq!(
        store.restore_operation_state(),
        RestoreOperationState::RecoveryRequired
    );

    fs::write(
        group
            .fixture
            .temp
            .path()
            .join("data/.longhorn/grouped-adapter-restore/journal.json"),
        b"{broken",
    )
    .unwrap();
    assert_eq!(
        store.restore_operation_state(),
        RestoreOperationState::RecoveryRequired
    );
    assert!(
        store
            .recover_grouped_adapter_restore(&catalog, Duration::from_secs(2))
            .is_err()
    );
}

#[test]
fn mixed_file_and_wal_sqlite_adapters_commit_one_group() {
    let fixture = Fixture::new();
    let file_domain = OpaqueDomain::new(
        "group.z-file",
        StorageClass::UserConfig,
        "group/file.json",
        &[],
    );
    let sqlite_domain = OpaqueDomain::new(
        "group.a-sqlite",
        StorageClass::MachineState,
        "group/sqlite.json",
        &[],
    );
    let source_file = fixture.temp.path().join("source-mixed.bin");
    let source_database = fixture.temp.path().join("source-mixed.sqlite3");
    fs::write(&source_file, b"mixed-new").unwrap();
    let source_connection = seed_wal_database(&source_database, "sqlite-new");
    let source_file_adapter = FileAdapter::new(source_file, FailureMode::None);
    let source_sqlite_adapter = SqliteAdapter::new(
        source_database,
        BackupAdapterRestoreParticipation::GroupedFailureAtomic,
        None,
    );
    let mut source_store = fixture.store();
    source_store.register(&file_domain).unwrap();
    source_store.register(&sqlite_domain).unwrap();
    let mut source_catalog = BackupCatalog::new();
    source_catalog
        .custom(&file_domain, &source_file_adapter)
        .unwrap();
    source_catalog
        .custom(&sqlite_domain, &source_sqlite_adapter)
        .unwrap();
    let snapshot = source_store
        .capture_backup(
            &source_catalog,
            &BackupScope::AllRegistered,
            group_metadata(),
            BackupCaptureOptions::new(Duration::from_secs(2), BackupLimits::default()),
        )
        .unwrap();
    let encoded = encode_backup_archive(&snapshot, BackupArchiveLimits::default()).unwrap();
    let archive = inspect_backup_archive(encoded.bytes(), BackupArchiveLimits::default()).unwrap();
    drop(source_connection);

    let target_file = fixture.temp.path().join("target-mixed.bin");
    let target_database = fixture.temp.path().join("target-mixed.sqlite3");
    fs::write(&target_file, b"mixed-old").unwrap();
    drop(seed_wal_database(&target_database, "sqlite-old"));
    let target_file_adapter = FileAdapter::new(target_file.clone(), FailureMode::None);
    let target_sqlite_adapter = SqliteAdapter::new(
        target_database.clone(),
        BackupAdapterRestoreParticipation::GroupedFailureAtomic,
        None,
    );
    let mut target_store = fixture.store();
    target_store.register(&file_domain).unwrap();
    target_store.register(&sqlite_domain).unwrap();
    let mut target_catalog = BackupCatalog::new();
    target_catalog
        .custom(&file_domain, &target_file_adapter)
        .unwrap();
    target_catalog
        .custom(&sqlite_domain, &target_sqlite_adapter)
        .unwrap();
    let inspection = inspect(&target_store, &target_catalog, &archive);
    let plan = target_store
        .plan_grouped_adapter_restore(
            &inspection,
            [
                file_domain.descriptor().id().clone(),
                sqlite_domain.descriptor().id().clone(),
            ],
        )
        .unwrap();
    target_store
        .execute_grouped_adapter_restore(
            &target_catalog,
            &archive,
            &inspection,
            &plan,
            plan.confirmation_digest(),
            group_options(),
        )
        .unwrap();
    assert_eq!(fs::read(&target_file).unwrap(), b"mixed-new");
    assert_eq!(database_value(&target_database), "sqlite-new");
    assert!(!sqlite_sidecar(&target_database, "-wal").is_file());

    drop(target_catalog);
    fs::write(&target_file, b"mixed-old").unwrap();
    drop(seed_wal_database(&target_database, "sqlite-old"));
    let rollback_file_adapter = FileAdapter::new(target_file.clone(), FailureMode::Apply);
    let rollback_sqlite_adapter = SqliteAdapter::new(
        target_database.clone(),
        BackupAdapterRestoreParticipation::GroupedFailureAtomic,
        None,
    );
    let mut rollback_catalog = BackupCatalog::new();
    rollback_catalog
        .custom(&file_domain, &rollback_file_adapter)
        .unwrap();
    rollback_catalog
        .custom(&sqlite_domain, &rollback_sqlite_adapter)
        .unwrap();
    let rollback_inspection = inspect(&target_store, &rollback_catalog, &archive);
    let rollback_plan = target_store
        .plan_grouped_adapter_restore(
            &rollback_inspection,
            [
                file_domain.descriptor().id().clone(),
                sqlite_domain.descriptor().id().clone(),
            ],
        )
        .unwrap();
    let error = target_store
        .execute_grouped_adapter_restore(
            &rollback_catalog,
            &archive,
            &rollback_inspection,
            &rollback_plan,
            rollback_plan.confirmation_digest(),
            group_options(),
        )
        .unwrap_err();
    assert_eq!(error.terminal(), RestoreFailureTerminal::RolledBack);
    assert_eq!(fs::read(&target_file).unwrap(), b"mixed-old");
    assert_eq!(database_value(&target_database), "sqlite-old");
}

fn inspect<'a>(
    store: &longhorn_config::ConfigStore,
    catalog: &BackupCatalog<'a>,
    archive: &longhorn_config::BackupArchiveInspection,
) -> longhorn_config::RestoreInspection {
    store.inspect_restore(
        catalog,
        archive,
        &BackupApplication::new("com.example.desktop", "9").unwrap(),
        &BackupProducer::new("longhorn-config", "9").unwrap(),
    )
}

fn plan(
    store: &longhorn_config::ConfigStore,
    inspection: &longhorn_config::RestoreInspection,
    group: &GroupFixture,
) -> longhorn_config::RestoreAdapterGroupPlan {
    store
        .plan_grouped_adapter_restore(
            inspection,
            [
                group.first.descriptor().id().clone(),
                group.second.descriptor().id().clone(),
            ],
        )
        .unwrap()
}

fn leave_interrupted_group(group: &GroupFixture) {
    let first = FileAdapter::new(group.first_path.clone(), FailureMode::None);
    let second = FileAdapter::new(group.second_path.clone(), FailureMode::PanicApply);
    let store = group.store();
    let mut catalog = BackupCatalog::new();
    catalog.custom(&group.first, &first).unwrap();
    catalog.custom(&group.second, &second).unwrap();
    let inspection = inspect(&store, &catalog, &group.archive);
    let plan = plan(&store, &inspection, group);
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = store.execute_grouped_adapter_restore(
                &catalog,
                &group.archive,
                &inspection,
                &plan,
                plan.confirmation_digest(),
                group_options(),
            );
        }))
        .is_err()
    );
}

fn group_metadata() -> BackupMetadata {
    BackupMetadata::new(
        "grouped-adapter-fixture",
        BackupKind::Operational,
        "2026-08-02T20:00:00Z",
        BackupApplication::new("com.example.desktop", "9").unwrap(),
        BackupProducer::new("longhorn-config", "9").unwrap(),
    )
    .unwrap()
}

fn group_options() -> RestoreAdapterGroupExecutionOptions {
    RestoreAdapterGroupExecutionOptions::new(Duration::from_secs(2), BackupLimits::default())
}
