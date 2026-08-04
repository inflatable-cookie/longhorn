use super::*;

pub(super) struct OpaqueDomain {
    descriptor: DomainDescriptor,
    required_fields: &'static [&'static str],
}

impl OpaqueDomain {
    pub(super) fn new(
        id: &str,
        class: StorageClass,
        path: &str,
        required_fields: &'static [&'static str],
    ) -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new(id).unwrap(),
                SchemaVersion::new(1).unwrap(),
                class,
                Some(DomainFilePath::new(path).unwrap()),
            )
            .unwrap(),
            required_fields,
        }
    }

    pub(super) fn path(&self, fixture: &Fixture) -> PathBuf {
        match fixture.roots.resolve(&self.descriptor) {
            longhorn_config::DomainLocation::File(file) => file.full_path().to_path_buf(),
            location => panic!("expected file domain, found {location:?}"),
        }
    }
}

impl ConfigDomain for OpaqueDomain {
    type Value = Value;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        self.required_fields
            .iter()
            .map(|field| ((*field).to_owned(), Value::Null))
            .collect()
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        Ok(value)
    }

    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        Ok(value.clone())
    }

    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue> {
        self.validate_raw(self.descriptor.schema_version(), value)
    }

    fn validate_raw(
        &self,
        _schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        let object = value
            .as_object()
            .ok_or_else(|| DomainIssue::new("shape", "fixture value must be an object"))?;
        if self
            .required_fields
            .iter()
            .all(|field| object.contains_key(*field))
        {
            Ok(())
        } else {
            Err(DomainIssue::new(
                "shape",
                "fixture is missing donor-owned fields",
            ))
        }
    }

    fn migrate_one(
        &self,
        _from: SchemaVersion,
        _value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        Ok(None)
    }
}

pub(super) struct SqliteAdapter {
    id: BackupAdapterId,
    capabilities: BackupAdapterCapabilities,
    database: PathBuf,
    coordination_probe: Option<CoordinationAuthority>,
    pub(super) captured_outside_longhorn_guard: AtomicBool,
}

impl SqliteAdapter {
    pub(super) fn new(
        database: PathBuf,
        participation: BackupAdapterRestoreParticipation,
        coordination_probe: Option<CoordinationAuthority>,
    ) -> Self {
        Self {
            id: BackupAdapterId::new("sqlite-online-v1").unwrap(),
            capabilities: BackupAdapterCapabilities::new(
                BackupAdapterCaptureMode::ExternalSnapshot(
                    BackupAdapterConsistencyGroup::new(
                        "soundcheck-sqlite",
                        "sqlite-online-backup-api",
                    )
                    .unwrap(),
                ),
                participation,
            ),
            database,
            coordination_probe,
            captured_outside_longhorn_guard: AtomicBool::new(false),
        }
    }

    fn snapshot_bytes(&self) -> Result<Vec<u8>, BackupAdapterError> {
        let scratch = tempdir().map_err(|_| adapter_failure("sqlite-scratch"))?;
        let snapshot = scratch.path().join("snapshot.sqlite3");
        let source =
            Connection::open(&self.database).map_err(|_| adapter_failure("sqlite-open-source"))?;
        source
            .backup(MAIN_DB, &snapshot, None)
            .map_err(|_| adapter_failure("sqlite-online-backup"))?;
        validate_database(&snapshot)?;
        fs::read(snapshot).map_err(|_| adapter_failure("sqlite-read-snapshot"))
    }

    fn payload_database(
        request: &BackupAdapterInspectRequest<'_>,
    ) -> Result<(TempDir, PathBuf), BackupAdapterError> {
        let [payload] = request.payloads() else {
            return Err(adapter_failure("sqlite-payload-count"));
        };
        if !payload.path().as_str().ends_with("/library.sqlite3") {
            return Err(adapter_failure("sqlite-payload-path"));
        }
        let scratch = tempdir().map_err(|_| adapter_failure("sqlite-scratch"))?;
        let path = scratch.path().join("library.sqlite3");
        fs::write(&path, payload.bytes()).map_err(|_| adapter_failure("sqlite-stage"))?;
        validate_database(&path)?;
        Ok((scratch, path))
    }
}

impl BackupAdapter for SqliteAdapter {
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
        if let Some(authority) = &self.coordination_probe {
            let lock = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(authority.lock_path())
                .map_err(|_| adapter_failure("longhorn-lock-open"))?;
            FileExt::try_lock(&lock).map_err(|_| adapter_failure("longhorn-guard-still-held"))?;
            self.captured_outside_longhorn_guard
                .store(true, Ordering::SeqCst);
        }
        if !self.database.is_file() {
            return Ok(BackupAdapterCapture::Absent);
        }
        let bytes = self.snapshot_bytes()?;
        if bytes.len() > request.limits().max_domain_bytes() {
            return Err(adapter_failure("sqlite-snapshot-too-large"));
        }
        Ok(BackupAdapterCapture::Present {
            source_schema_version: request.descriptor().schema_version(),
            payloads: vec![BackupAdapterPayload::new(
                BackupAdapterRelativePath::new("library.sqlite3").unwrap(),
                bytes,
            )],
        })
    }

    fn inspect(
        &self,
        request: BackupAdapterInspectRequest<'_>,
    ) -> Result<BackupAdapterRestorePreview, BackupAdapterError> {
        let current = self
            .database
            .is_file()
            .then(|| semantic_digest(&self.database))
            .transpose()?;
        let target = match request.source_state() {
            BackupSourceState::Absent if request.payloads().is_empty() => {
                BackupAdapterStateEvidence::Absent
            }
            BackupSourceState::Present => {
                let (_scratch, path) = Self::payload_database(&request)?;
                BackupAdapterStateEvidence::present(semantic_digest(&path)?)
            }
            _ => return Err(adapter_failure("sqlite-source-state")),
        };
        Ok(BackupAdapterRestorePreview::new(
            target,
            BackupAdapterStateEvidence::from_optional(current),
        ))
    }

    fn restore(
        &self,
        request: BackupAdapterRestoreRequest<'_>,
    ) -> Result<BackupAdapterRestoreOutcome, BackupAdapterError> {
        let (_scratch, source) = Self::payload_database(request.inspect())?;
        if Some(&semantic_digest(&source)?) != request.preview().target_evidence().sha256() {
            return Err(adapter_failure("sqlite-preview-changed"));
        }
        let mut destination = Connection::open(&self.database)
            .map_err(|_| adapter_failure("sqlite-open-destination"))?;
        destination
            .restore(MAIN_DB, &source, None::<fn(rusqlite::backup::Progress)>)
            .map_err(|_| adapter_failure("sqlite-restore"))?;
        drop(destination);
        validate_database(&self.database)?;
        Ok(BackupAdapterRestoreOutcome::Verified {
            evidence: semantic_digest(&self.database)?,
        })
    }

    fn grouped_restore(&self) -> Option<&dyn BackupAdapterGroupedRestore> {
        (self.capabilities.restore() == &BackupAdapterRestoreParticipation::GroupedFailureAtomic)
            .then_some(self)
    }
}

impl BackupAdapterGroupedRestore for SqliteAdapter {
    fn stage(
        &self,
        request: BackupAdapterGroupedStageRequest<'_>,
    ) -> Result<BackupAdapterRestoreStage, BackupAdapterError> {
        let target_payloads = match request.preview().target_evidence() {
            BackupAdapterStateEvidence::Absent => Vec::new(),
            BackupAdapterStateEvidence::Present { .. } => {
                let [target] = request.inspect().payloads() else {
                    return Err(adapter_failure("sqlite-group-target-count"));
                };
                if target.bytes().len() > request.limits().max_domain_bytes() {
                    return Err(adapter_failure("sqlite-group-target-too-large"));
                }
                let (_scratch, target_path) = Self::payload_database(request.inspect())?;
                if Some(&semantic_digest(&target_path)?)
                    != request.preview().target_evidence().sha256()
                {
                    return Err(adapter_failure("sqlite-group-target-evidence"));
                }
                vec![BackupAdapterPayload::new(
                    BackupAdapterRelativePath::new("library.sqlite3").unwrap(),
                    target.bytes().to_vec(),
                )]
            }
        };
        let rollback_payloads = if self.database.is_file() {
            vec![BackupAdapterPayload::new(
                BackupAdapterRelativePath::new("library.sqlite3").unwrap(),
                self.snapshot_bytes()?,
            )]
        } else {
            Vec::new()
        };
        Ok(BackupAdapterRestoreStage::new(
            target_payloads,
            rollback_payloads,
            request.preview().target_evidence().clone(),
            request.preview().current_evidence().clone(),
        ))
    }

    fn apply(
        &self,
        request: BackupAdapterGroupedApplyRequest<'_>,
    ) -> Result<(), BackupAdapterError> {
        if request.expected_evidence().is_absent() {
            match fs::remove_file(&self.database) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(adapter_failure("sqlite-group-remove")),
            }
            for suffix in ["-wal", "-shm"] {
                let _ = fs::remove_file(sqlite_sidecar(&self.database, suffix));
            }
            return Ok(());
        };
        let [payload] = request.payloads() else {
            return Err(adapter_failure("sqlite-group-apply-count"));
        };
        let scratch = tempdir().map_err(|_| adapter_failure("sqlite-group-scratch"))?;
        let source = scratch.path().join("library.sqlite3");
        fs::write(&source, payload.bytes())
            .map_err(|_| adapter_failure("sqlite-group-write-stage"))?;
        validate_database(&source)?;
        let mut destination = Connection::open(&self.database)
            .map_err(|_| adapter_failure("sqlite-group-open-destination"))?;
        destination
            .restore(MAIN_DB, &source, None::<fn(rusqlite::backup::Progress)>)
            .map_err(|_| adapter_failure("sqlite-group-restore"))?;
        drop(destination);
        validate_database(&self.database)
    }

    fn verify(
        &self,
        _request: BackupAdapterGroupedVerifyRequest<'_>,
    ) -> Result<BackupAdapterStateEvidence, BackupAdapterError> {
        Ok(BackupAdapterStateEvidence::from_optional(
            self.database
                .is_file()
                .then(|| semantic_digest(&self.database))
                .transpose()?,
        ))
    }
}

pub(super) struct StaticAdapter {
    id: BackupAdapterId,
    capabilities: BackupAdapterCapabilities,
    payloads: Vec<BackupAdapterPayload>,
}

impl StaticAdapter {
    pub(super) fn new(payloads: Vec<BackupAdapterPayload>) -> Self {
        Self {
            id: BackupAdapterId::new("static-fixture-v1").unwrap(),
            capabilities: BackupAdapterCapabilities::new(
                BackupAdapterCaptureMode::CoordinatedBounded,
                BackupAdapterRestoreParticipation::Separate,
            ),
            payloads,
        }
    }
}

impl BackupAdapter for StaticAdapter {
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
        Ok(BackupAdapterCapture::Present {
            source_schema_version: request.descriptor().schema_version(),
            payloads: self.payloads.clone(),
        })
    }

    fn inspect(
        &self,
        request: BackupAdapterInspectRequest<'_>,
    ) -> Result<BackupAdapterRestorePreview, BackupAdapterError> {
        let bytes = request
            .payloads()
            .iter()
            .flat_map(|payload| payload.bytes())
            .copied()
            .collect::<Vec<_>>();
        Ok(BackupAdapterRestorePreview::new(
            BackupAdapterStateEvidence::present(Sha256Digest::from_bytes(&bytes)),
            BackupAdapterStateEvidence::Absent,
        ))
    }

    fn restore(
        &self,
        request: BackupAdapterRestoreRequest<'_>,
    ) -> Result<BackupAdapterRestoreOutcome, BackupAdapterError> {
        Ok(BackupAdapterRestoreOutcome::Verified {
            evidence: request
                .preview()
                .target_evidence()
                .sha256()
                .expect("static adapter target is present")
                .clone(),
        })
    }
}

pub(super) fn safety_options(fixture: &Fixture) -> RestoreSafetyBackupOptions {
    RestoreSafetyBackupOptions::new(
        BackupMetadata::new(
            "pre-restore-donor-fixtures",
            BackupKind::PreRestore,
            "2026-07-28T14:00:00Z",
            BackupApplication::new("com.example.desktop", "9").unwrap(),
            BackupProducer::new("longhorn-config", "9").unwrap(),
        )
        .unwrap(),
        BackupOperationalRoot::new(fixture.temp.path().join("backups")).unwrap(),
        BackupArchiveFileName::new("pre-restore-donor-fixtures.longhorn-backup").unwrap(),
        super::super::options(longhorn_config::BackupLimits::default()),
        BackupArchiveLimits::default(),
    )
}

pub(super) fn seed_wal_database(path: &Path, value: &str) -> Connection {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA wal_autocheckpoint = 0;
             CREATE TABLE IF NOT EXISTS items (
                 id INTEGER PRIMARY KEY,
                 value TEXT NOT NULL
             );",
        )
        .unwrap();
    connection
        .execute(
            "INSERT OR REPLACE INTO items (id, value) VALUES (1, ?1)",
            params![value],
        )
        .unwrap();
    connection
}

fn validate_database(path: &Path) -> Result<(), BackupAdapterError> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| adapter_failure("sqlite-open-snapshot"))?;
    let check = connection
        .query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
        .map_err(|_| adapter_failure("sqlite-quick-check"))?;
    if check != "ok" {
        return Err(adapter_failure("sqlite-quick-check-failed"));
    }
    connection
        .query_row("SELECT COUNT(*) FROM items", [], |row| row.get::<_, i64>(0))
        .map_err(|_| adapter_failure("sqlite-schema"))?;
    Ok(())
}

pub(super) fn semantic_digest(path: &Path) -> Result<Sha256Digest, BackupAdapterError> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| adapter_failure("sqlite-open-semantic"))?;
    let mut statement = connection
        .prepare("SELECT id, value FROM items ORDER BY id")
        .map_err(|_| adapter_failure("sqlite-prepare-semantic"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| adapter_failure("sqlite-query-semantic"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| adapter_failure("sqlite-read-semantic"))?;
    let bytes = serde_json::to_vec(&rows).expect("semantic rows serialize");
    Ok(Sha256Digest::from_bytes(&bytes))
}

pub(super) fn database_value(path: &Path) -> String {
    Connection::open(path)
        .unwrap()
        .query_row("SELECT value FROM items WHERE id = 1", [], |row| row.get(0))
        .unwrap()
}

pub(super) fn sqlite_sidecar(path: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{suffix}", path.display()))
}

pub(super) fn adapter_failure(code: &str) -> BackupAdapterError {
    BackupAdapterError::failed(code).unwrap()
}
