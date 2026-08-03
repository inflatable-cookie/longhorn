use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use longhorn_config::{
    BackupAdapter, BackupAdapterCapabilities, BackupAdapterCapture, BackupAdapterCaptureMode,
    BackupAdapterCaptureRequest, BackupAdapterConsistencyGroup, BackupAdapterError,
    BackupAdapterId, BackupAdapterInspectRequest, BackupAdapterPayload, BackupAdapterRelativePath,
    BackupAdapterRestoreOutcome, BackupAdapterRestoreParticipation, BackupAdapterRestorePreview,
    BackupAdapterRestoreRequest, BackupAdapterStateEvidence, ConfigDomain, Sha256Digest,
    StorageClass, StorageTransitionAction, StorageTransitionAdapter, StorageTransitionCatalog,
    StorageTransitionExecutionOptions, StorageTransitionGuard, StorageTransitionOutcome,
    StorageTransitionRequest, execute_storage_transition, inspect_storage_transition,
    plan_storage_transition,
};
use rusqlite::{Connection, DatabaseName, OpenFlags, params};
use tempfile::{TempDir, tempdir};

use crate::transition_support::{TestDomain, TransitionFixture};

struct SqliteTransitionAdapter {
    id: BackupAdapterId,
    capabilities: BackupAdapterCapabilities,
    database: PathBuf,
    authority: String,
    gate: Mutex<()>,
}

impl SqliteTransitionAdapter {
    fn new(database: PathBuf, authority: &str) -> Self {
        Self {
            id: BackupAdapterId::new("sqlite-transition-v1").unwrap(),
            capabilities: BackupAdapterCapabilities::new(
                BackupAdapterCaptureMode::ExternalSnapshot(
                    BackupAdapterConsistencyGroup::new(authority, "sqlite-transition-guard")
                        .unwrap(),
                ),
                BackupAdapterRestoreParticipation::Separate,
            ),
            database,
            authority: authority.into(),
            gate: Mutex::new(()),
        }
    }

    fn payload_database(
        request: &BackupAdapterInspectRequest<'_>,
    ) -> Result<(TempDir, PathBuf), BackupAdapterError> {
        let [payload] = request.payloads() else {
            return Err(failure("sqlite-payload-count"));
        };
        let scratch = tempdir().map_err(|_| failure("sqlite-scratch"))?;
        let path = scratch.path().join("database.sqlite3");
        fs::write(&path, payload.bytes()).map_err(|_| failure("sqlite-stage"))?;
        validate_database(&path)?;
        Ok((scratch, path))
    }
}

impl BackupAdapter for SqliteTransitionAdapter {
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
        let scratch = tempdir().map_err(|_| failure("sqlite-scratch"))?;
        let snapshot = scratch.path().join("snapshot.sqlite3");
        let source = Connection::open(&self.database).map_err(|_| failure("sqlite-open-source"))?;
        source
            .backup(DatabaseName::Main, &snapshot, None)
            .map_err(|_| failure("sqlite-backup"))?;
        validate_database(&snapshot)?;
        let bytes = fs::read(snapshot).map_err(|_| failure("sqlite-read-snapshot"))?;
        if bytes.len() > request.limits().max_domain_bytes() {
            return Err(failure("sqlite-snapshot-too-large"));
        }
        Ok(BackupAdapterCapture::Present {
            source_schema_version: request.descriptor().schema_version(),
            payloads: vec![BackupAdapterPayload::new(
                BackupAdapterRelativePath::new("database.sqlite3").unwrap(),
                bytes,
            )],
        })
    }

    fn inspect(
        &self,
        request: BackupAdapterInspectRequest<'_>,
    ) -> Result<BackupAdapterRestorePreview, BackupAdapterError> {
        let (_scratch, staged) = Self::payload_database(&request)?;
        Ok(BackupAdapterRestorePreview::new(
            BackupAdapterStateEvidence::present(semantic_digest(&staged)?),
            BackupAdapterStateEvidence::from_optional(
                self.database
                    .is_file()
                    .then(|| semantic_digest(&self.database))
                    .transpose()?,
            ),
        ))
    }

    fn restore(
        &self,
        request: BackupAdapterRestoreRequest<'_>,
    ) -> Result<BackupAdapterRestoreOutcome, BackupAdapterError> {
        let (_scratch, staged) = Self::payload_database(request.inspect())?;
        if Some(&semantic_digest(&staged)?) != request.preview().target_evidence().sha256() {
            return Err(failure("sqlite-preview-changed"));
        }
        if let Some(parent) = self.database.parent() {
            fs::create_dir_all(parent).map_err(|_| failure("sqlite-create-parent"))?;
        }
        let mut target =
            Connection::open(&self.database).map_err(|_| failure("sqlite-open-target"))?;
        target
            .restore(
                DatabaseName::Main,
                &staged,
                None::<fn(rusqlite::backup::Progress)>,
            )
            .map_err(|_| failure("sqlite-restore"))?;
        drop(target);
        Ok(BackupAdapterRestoreOutcome::Verified {
            evidence: semantic_digest(&self.database)?,
        })
    }
}

impl StorageTransitionAdapter for SqliteTransitionAdapter {
    fn transition_authority(&self) -> &str {
        &self.authority
    }

    fn acquire_transition_guard(
        &self,
        _descriptor: &longhorn_config::DomainDescriptor,
        _timeout: Duration,
    ) -> Result<Box<dyn StorageTransitionGuard + '_>, BackupAdapterError> {
        self.gate
            .lock()
            .map(|guard| Box::new(guard) as Box<dyn StorageTransitionGuard>)
            .map_err(|_| failure("sqlite-transition-lock"))
    }

    fn owned_paths(&self, _descriptor: &longhorn_config::DomainDescriptor) -> Vec<PathBuf> {
        vec![
            self.database.clone(),
            sqlite_sidecar(&self.database, "-wal"),
            sqlite_sidecar(&self.database, "-shm"),
        ]
    }

    fn current_evidence(
        &self,
        _descriptor: &longhorn_config::DomainDescriptor,
    ) -> Result<Option<Sha256Digest>, BackupAdapterError> {
        self.database
            .is_file()
            .then(|| semantic_digest(&self.database))
            .transpose()
    }
}

#[test]
fn sqlite_transition_uses_native_snapshot_and_never_copies_main_or_wal_files() {
    let fixture = TransitionFixture::new();
    let domain = TestDomain::new(
        "soundcheck.library",
        StorageClass::UserConfig,
        "soundcheck/library-authority.json",
    );
    let source_path = fixture
        .source
        .durable_database_dir()
        .join("library.sqlite3");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    let source_connection = seed_database(&source_path, "wal-value");
    let wal_path = sqlite_sidecar(&source_path, "-wal");
    let wal_before = fs::read(&wal_path).unwrap();
    let target_path = fixture
        .target
        .durable_database_dir()
        .join("library.sqlite3");
    let source_adapter =
        SqliteTransitionAdapter::new(source_path.clone(), "soundcheck-source-sqlite");
    let target_adapter =
        SqliteTransitionAdapter::new(target_path.clone(), "soundcheck-target-sqlite");

    let mut source_store = fixture.store(&fixture.source);
    let mut target_store = fixture.store(&fixture.target);
    source_store.register(&domain).unwrap();
    target_store.register(&domain).unwrap();
    let mut catalog = StorageTransitionCatalog::new();
    catalog
        .custom(&domain, &source_adapter, &target_adapter)
        .unwrap();
    let request = StorageTransitionRequest::new(
        &source_store,
        &target_store,
        &fixture.source,
        &fixture.target,
        fixture.target_selection.clone(),
        &catalog,
        fixture.bootstrap(),
    );
    let preview = inspect_storage_transition(&request).unwrap();
    assert!(preview.conflicts().is_empty());
    assert!(preview.source_unknown().is_empty());
    assert!(matches!(
        preview.domains()[0].action(),
        StorageTransitionAction::CustomAdapter { .. }
    ));
    let plan = plan_storage_transition(&preview).unwrap();
    let receipt = execute_storage_transition(
        &request,
        &plan,
        plan.confirmation_digest(),
        StorageTransitionExecutionOptions::new("sqlite-transition", Duration::from_secs(2))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(receipt.outcome(), StorageTransitionOutcome::TargetCommitted);
    assert_eq!(receipt.custom_domains(), [domain.descriptor().id().clone()]);
    assert!(receipt.copied_domains().is_empty());
    assert_eq!(database_value(&target_path), "wal-value");
    assert_eq!(fs::read(&wal_path).unwrap(), wal_before);
    assert!(!domain.path(&fixture.target).exists());
    drop(source_connection);
}

fn seed_database(path: &Path, value: &str) -> Connection {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA wal_autocheckpoint = 0;
             CREATE TABLE IF NOT EXISTS values_table (
                 id INTEGER PRIMARY KEY,
                 value TEXT NOT NULL
             );",
        )
        .unwrap();
    connection
        .execute(
            "INSERT OR REPLACE INTO values_table (id, value) VALUES (1, ?1)",
            params![value],
        )
        .unwrap();
    connection
}

fn validate_database(path: &Path) -> Result<(), BackupAdapterError> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| failure("sqlite-open-validate"))?;
    let check = connection
        .query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
        .map_err(|_| failure("sqlite-quick-check"))?;
    if check != "ok" {
        return Err(failure("sqlite-invalid"));
    }
    Ok(())
}

fn semantic_digest(path: &Path) -> Result<Sha256Digest, BackupAdapterError> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| failure("sqlite-open-semantic"))?;
    let value = connection
        .query_row("SELECT value FROM values_table WHERE id = 1", [], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|_| failure("sqlite-read-semantic"))?;
    Ok(Sha256Digest::from_bytes(value.as_bytes()))
}

fn database_value(path: &Path) -> String {
    Connection::open(path)
        .unwrap()
        .query_row("SELECT value FROM values_table WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap()
}

fn sqlite_sidecar(path: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{suffix}", path.display()))
}

fn failure(code: &str) -> BackupAdapterError {
    BackupAdapterError::failed(code).unwrap()
}
