//! Human-written TOML domains (contract 004, phase 1), through the public API
//! with the `toml` feature: read, validate, in-memory migration, line/column
//! error reports, read-only refusals, and backup exclusion.

#![cfg(feature = "toml")]

use std::{fs, path::PathBuf, time::Duration};

use longhorn_config::{
    BackupApplication, BackupArchiveFileName, BackupArchiveLimits, BackupCaptureError,
    BackupCaptureOptions, BackupCatalog, BackupExclusion, BackupKind, BackupLimits, BackupMetadata,
    BackupOperationalRoot, BackupProducer, BackupScope, ConfigDomain, ConfigStore,
    CoordinationAuthority, DebounceClock, DebouncePolicy, DebounceStrategy, DebouncedMutation,
    DomainDescriptor, DomainFilePath, DomainIssue, DomainLocation, DurabilityRequirement,
    LoadDiagnosticCode, LoadOutcome, LoadedOrigin, MigrationRewriteError, MigrationRewriteOptions,
    MigrationStep, MutationError, MutationOptions, MutationRefusal, RecoveryKind,
    RestoreDomainCompatibility, RestoreSafetyBackupOptions, SourcePosition, StorageClass,
    encode_backup_archive, inspect_backup_archive,
};
use longhorn_core::{DomainId, SchemaVersion};
use longhorn_storage_layout::StorageRoots;
use serde_json::{Value, json};
use tempfile::TempDir;

const USER_CONFIG_CLASS: StorageClass = StorageClass::UserConfig;

// --- fixture -------------------------------------------------------------

struct Fixture {
    /// Holds the scratch directory alive for the whole fixture lifetime.
    #[allow(dead_code)]
    temp: TempDir,
    config: PathBuf,
    backups: PathBuf,
    roots: StorageRoots,
    coordination: CoordinationAuthority,
}

impl Fixture {
    fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let config = root.join("config");
        let data = root.join("data");
        let backups = root.join("backups");
        for path in [&config, &data, &backups] {
            fs::create_dir_all(path).unwrap();
        }
        let coordination = CoordinationAuthority::new(&data).unwrap();
        let roots = StorageRoots::new(
            config.clone(),
            data,
            root.join("state"),
            root.join("cache"),
            root.join("runtime"),
            root.join("logs"),
            backups.clone(),
        )
        .unwrap();
        Self {
            temp,
            config,
            backups,
            roots,
            coordination,
        }
    }

    fn store(&self) -> ConfigStore {
        ConfigStore::new(self.roots.clone(), self.coordination.clone())
    }

    fn write_config(&self, file_name: &str, contents: &str) -> PathBuf {
        let path = self.config.join(file_name);
        fs::write(&path, contents).unwrap();
        path
    }

    fn read_config(&self, file_name: &str) -> Vec<u8> {
        fs::read(self.config.join(file_name)).unwrap()
    }
}

// --- strict settings domain (schema 1 -> 2) ------------------------------

struct SettingsDomain {
    descriptor: DomainDescriptor,
}

impl SettingsDomain {
    fn new() -> Self {
        Self {
            descriptor: DomainDescriptor::new_toml(
                DomainId::new("example.cli.settings").unwrap(),
                SchemaVersion::new(2).unwrap(),
                USER_CONFIG_CLASS,
                Some(DomainFilePath::new_toml("cli-settings.toml").unwrap()),
            )
            .unwrap(),
        }
    }
}

impl ConfigDomain for SettingsDomain {
    type Value = Value;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        json!({"name": "default", "enabled": true})
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        Ok(value)
    }

    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        Ok(value.clone())
    }

    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue> {
        if value.get("name").is_some_and(Value::is_string)
            && value.get("enabled").is_some_and(Value::is_boolean)
        {
            Ok(())
        } else {
            Err(DomainIssue::new("shape", "name and enabled are required"))
        }
    }

    fn validate_raw(
        &self,
        schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        let Some(object) = value.as_object() else {
            return Err(DomainIssue::new("shape", "value must be a table"));
        };
        let valid = match schema_version.get() {
            1 => object.get("label").is_some_and(Value::is_string),
            2 => {
                object.get("name").is_some_and(Value::is_string)
                    && object.get("enabled").is_some_and(Value::is_boolean)
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(DomainIssue::new(
                "schema-shape",
                format!("invalid schema {} value", schema_version.get()),
            ))
        }
    }

    fn migrate_one(
        &self,
        from: SchemaVersion,
        mut value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        if from.get() != 1 {
            return Ok(None);
        }
        let object = value
            .as_object_mut()
            .ok_or_else(|| DomainIssue::new("shape", "value must be a table"))?;
        let label = object
            .remove("label")
            .ok_or_else(|| DomainIssue::new("label", "label is required"))?;
        object.insert("name".to_owned(), label);
        object.insert("enabled".to_owned(), Value::Bool(true));
        Ok(Some(MigrationStep {
            schema_version: SchemaVersion::new(2).unwrap(),
            value,
        }))
    }
}

// --- permissive domain for type-mapping edge cases -----------------------

/// A human TOML domain in the policy class. The fixture injects no policy
/// root, so it resolves to `RootRequired` and every write path must refuse
/// instead of assuming a file location.
struct PolicyTomlDomain {
    descriptor: DomainDescriptor,
}

impl PolicyTomlDomain {
    fn new() -> Self {
        Self {
            descriptor: DomainDescriptor::new_toml(
                DomainId::new("example.cli.policy").unwrap(),
                SchemaVersion::new(1).unwrap(),
                StorageClass::Policy,
                Some(DomainFilePath::new_toml("cli-policy.toml").unwrap()),
            )
            .unwrap(),
        }
    }
}

impl ConfigDomain for PolicyTomlDomain {
    type Value = Value;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        json!({})
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
        value: &Value,
    ) -> Result<(), DomainIssue> {
        if value.is_object() {
            Ok(())
        } else {
            Err(DomainIssue::new("shape", "value must be a table"))
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

struct FreeformDomain {
    descriptor: DomainDescriptor,
}

impl FreeformDomain {
    fn new(file_name: &str) -> Self {
        Self {
            descriptor: DomainDescriptor::new_toml(
                DomainId::new(format!("example.cli.{file_name}").replace(".toml", "")).unwrap(),
                SchemaVersion::new(1).unwrap(),
                USER_CONFIG_CLASS,
                Some(DomainFilePath::new_toml(file_name).unwrap()),
            )
            .unwrap(),
        }
    }
}

impl ConfigDomain for FreeformDomain {
    type Value = Value;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        json!({})
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
        value: &Value,
    ) -> Result<(), DomainIssue> {
        if value.is_object() {
            Ok(())
        } else {
            Err(DomainIssue::new("shape", "value must be a table"))
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

// --- read paths -----------------------------------------------------------

#[test]
fn reads_validates_and_decodes_a_current_file() {
    let fixture = Fixture::new();
    fixture.write_config(
        "cli-settings.toml",
        "schema_version = 2\nname = \"cli\"\nenabled = false\n",
    );
    let domain = SettingsDomain::new();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let outcome = store.load(&domain).unwrap();
    let LoadOutcome::Ready(loaded) = outcome else {
        panic!("expected a ready load");
    };
    assert_eq!(loaded.value, json!({"name": "cli", "enabled": false}));
    assert_eq!(loaded.origin, LoadedOrigin::File);
    assert_eq!(loaded.schema_version.get(), 2);
    assert!(loaded.diagnostics.is_empty());
}

#[test]
fn missing_file_yields_the_default_and_missing_diagnostic() {
    let fixture = Fixture::new();
    let domain = SettingsDomain::new();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let outcome = store.load(&domain).unwrap();
    let LoadOutcome::Ready(loaded) = outcome else {
        panic!("expected a ready load");
    };
    assert_eq!(loaded.value, domain.default_value());
    assert_eq!(loaded.origin, LoadedOrigin::Default);
    assert_eq!(loaded.diagnostics.len(), 1);
    assert_eq!(loaded.diagnostics[0].code, LoadDiagnosticCode::Missing);
}

#[test]
fn missing_schema_version_means_the_current_version() {
    let fixture = Fixture::new();
    fixture.write_config("cli-settings.toml", "name = \"cli\"\nenabled = true\n");
    let domain = SettingsDomain::new();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let outcome = store.load(&domain).unwrap();
    let LoadOutcome::Ready(loaded) = outcome else {
        panic!("expected a ready load");
    };
    assert_eq!(loaded.schema_version.get(), 2);
    assert_eq!(loaded.origin, LoadedOrigin::File);
    assert_eq!(loaded.value, json!({"name": "cli", "enabled": true}));
}

#[test]
fn older_versions_migrate_in_memory_and_leave_the_file_bytes_alone() {
    let fixture = Fixture::new();
    let original = "schema_version = 1\nlabel = \"legacy\"\n";
    let path = fixture.write_config("cli-settings.toml", original);
    let domain = SettingsDomain::new();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let outcome = store.load(&domain).unwrap();
    let LoadOutcome::Ready(loaded) = outcome else {
        panic!("expected a ready load");
    };
    assert_eq!(
        loaded.origin,
        LoadedOrigin::MigratedInMemory {
            from: SchemaVersion::new(1).unwrap(),
            to: SchemaVersion::new(2).unwrap(),
        }
    );
    assert_eq!(loaded.value, json!({"name": "legacy", "enabled": true}));
    assert_eq!(fs::read(path).unwrap(), original.as_bytes());
}

#[test]
fn future_versions_fail_safe_and_keep_the_file() {
    let fixture = Fixture::new();
    let original = "schema_version = 3\nname = \"cli\"\nenabled = true\n";
    let path = fixture.write_config("cli-settings.toml", original);
    let domain = SettingsDomain::new();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let outcome = store.load(&domain).unwrap();
    let LoadOutcome::Recovery(recovery) = outcome else {
        panic!("expected a recovery state");
    };
    assert_eq!(recovery.kind, RecoveryKind::FutureSchema);
    assert_eq!(
        recovery.position,
        Some(SourcePosition {
            line: 1,
            column: 18
        })
    );
    assert_eq!(fs::read(path).unwrap(), original.as_bytes());
}

#[test]
fn syntax_errors_report_line_and_column() {
    let fixture = Fixture::new();
    fixture.write_config("cli-settings.toml", "name = \"cli\"\nenabled = \n");
    let domain = SettingsDomain::new();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let outcome = store.load(&domain).unwrap();
    let LoadOutcome::Recovery(recovery) = outcome else {
        panic!("expected a recovery state");
    };
    assert_eq!(recovery.kind, RecoveryKind::CorruptDocument);
    assert!(
        recovery.detail.contains("TOML parse error"),
        "unexpected detail: {}",
        recovery.detail
    );
    let position = recovery.position.expect("parse errors carry a position");
    assert_eq!(position.line, 2);
    assert!(position.column >= 1);
    assert_eq!(
        recovery.source.expect("source bytes kept").bytes,
        b"name = \"cli\"\nenabled = \n"
    );
}

#[test]
fn validation_errors_report_the_document_start() {
    let fixture = Fixture::new();
    fixture.write_config(
        "cli-settings.toml",
        "schema_version = 2\nname = 5\nenabled = true\n",
    );
    let domain = SettingsDomain::new();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let outcome = store.load(&domain).unwrap();
    let LoadOutcome::Recovery(recovery) = outcome else {
        panic!("expected a recovery state");
    };
    assert_eq!(recovery.kind, RecoveryKind::InvalidValue);
    assert_eq!(
        recovery.position,
        Some(SourcePosition { line: 1, column: 1 })
    );
}

#[test]
fn a_file_broken_after_a_good_load_keeps_the_last_good_value_and_the_bytes() {
    let fixture = Fixture::new();
    let good = "name = \"cli\"\nenabled = true\n";
    fixture.write_config("cli-settings.toml", good);
    let domain = SettingsDomain::new();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let LoadOutcome::Ready(first) = store.load(&domain).unwrap() else {
        panic!("expected the first load to succeed");
    };
    assert_eq!(first.value, json!({"name": "cli", "enabled": true}));

    let broken = "name = \"cli\"\nenabled = {\n";
    fixture.write_config("cli-settings.toml", broken);

    // The broken reload is a recovery state, never a silent default reset.
    let outcome = store.load(&domain).unwrap();
    let LoadOutcome::Recovery(recovery) = outcome else {
        panic!("expected a recovery state after the file broke");
    };
    assert_eq!(recovery.kind, RecoveryKind::CorruptDocument);
    assert_eq!(
        recovery.source.expect("source bytes kept").bytes,
        broken.as_bytes()
    );
    // The store wrote nothing: the file still holds exactly the broken
    // bytes, so the last good value was never destroyed.
    assert_eq!(fixture.read_config("cli-settings.toml"), broken.as_bytes());

    // The person repairs the file; the next load returns the same good
    // value the session held before the break.
    fixture.write_config("cli-settings.toml", good);
    let outcome = store.load(&domain).unwrap();
    let LoadOutcome::Ready(repaired) = outcome else {
        panic!("expected the repaired file to load");
    };
    assert_eq!(repaired.value, first.value);
    assert_eq!(repaired.origin, LoadedOrigin::File);
}

// --- read-only refusals ---------------------------------------------------

fn mutation_options() -> MutationOptions {
    MutationOptions::new(Duration::from_secs(1), DurabilityRequirement::Atomic)
}

#[test]
fn every_mutation_path_refuses_a_toml_domain() {
    let fixture = Fixture::new();
    let original = "name = \"cli\"\nenabled = true\n";
    fixture.write_config("cli-settings.toml", original);
    let domain = SettingsDomain::new();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let expected = MutationRefusal::HumanWrittenToml {
        domain: domain.descriptor().id().clone(),
        path: fixture.config.join("cli-settings.toml"),
    };

    let refused = store
        .mutate(&domain, mutation_options(), |value| {
            value["enabled"] = Value::Bool(false);
            Ok(())
        })
        .unwrap_err();
    assert_eq!(refused, MutationError::Refused(expected.clone()));

    let checked = store.mutate_checked(
        &domain,
        mutation_options(),
        |_| -> Result<(), std::convert::Infallible> { Ok(()) },
    );
    assert!(matches!(
        checked,
        Err(longhorn_config::CheckedMutationError::Mutation(
            MutationError::Refused(MutationRefusal::HumanWrittenToml { .. })
        ))
    ));

    assert_eq!(
        fixture.read_config("cli-settings.toml"),
        original.as_bytes()
    );
}

#[test]
fn a_toml_domain_without_an_injected_root_refuses_instead_of_panicking() {
    let fixture = Fixture::new();
    let domain = PolicyTomlDomain::new();
    let mut store = fixture.store();
    // The fixture injects no policy root, so the domain resolves to
    // RootRequired; registration still succeeds.
    store.register(&domain).unwrap();

    let refused = store
        .mutate(&domain, mutation_options(), |value| {
            value["enabled"] = Value::Bool(false);
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(
        refused,
        MutationError::Refused(MutationRefusal::Unavailable {
            location: DomainLocation::RootRequired { .. }
        })
    ));

    let checked = store.mutate_checked(
        &domain,
        mutation_options(),
        |_| -> Result<(), std::convert::Infallible> { Ok(()) },
    );
    assert!(matches!(
        checked,
        Err(longhorn_config::CheckedMutationError::Mutation(
            MutationError::Refused(MutationRefusal::Unavailable { .. })
        ))
    ));
}

#[test]
fn destructive_migration_rewrite_refuses_a_toml_domain() {
    let fixture = Fixture::new();
    let original = "schema_version = 1\nlabel = \"legacy\"\n";
    fixture.write_config("cli-settings.toml", original);
    let domain = SettingsDomain::new();
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut catalog = BackupCatalog::new();
    catalog
        .exclude(
            &domain,
            longhorn_config::BackupExclusionReason::new("human-written, read-only in phase 1")
                .unwrap(),
        )
        .unwrap();

    let error = store
        .rewrite_migrated_domain(
            &catalog,
            &domain,
            MigrationRewriteOptions::new(
                Duration::from_secs(2),
                safety_options(&fixture, "pre-migration-toml"),
            ),
        )
        .unwrap_err();
    assert_eq!(
        error,
        MigrationRewriteError::HumanWrittenToml {
            domain: domain.descriptor().id().clone(),
        }
    );
    assert_eq!(
        fixture.read_config("cli-settings.toml"),
        original.as_bytes()
    );
}

fn safety_options(fixture: &Fixture, archive_id: &str) -> RestoreSafetyBackupOptions {
    RestoreSafetyBackupOptions::new(
        BackupMetadata::new(
            archive_id,
            BackupKind::PreMigration,
            "2026-10-22T12:00:00Z",
            BackupApplication::new("com.example.cli", "1.0.0").unwrap(),
            BackupProducer::new("longhorn-config", "0.1.0").unwrap(),
        )
        .unwrap(),
        BackupOperationalRoot::new(fixture.backups.clone()).unwrap(),
        BackupArchiveFileName::new(format!("{archive_id}.longhorn-backup")).unwrap(),
        BackupCaptureOptions::new(Duration::from_secs(2), BackupLimits::default()),
        BackupArchiveLimits::default(),
    )
}

// --- debounce -------------------------------------------------------------

struct MergeStrategy;

impl DebounceStrategy<SettingsDomain> for MergeStrategy {
    type Intent = Value;

    fn coalesce(
        &self,
        previous: &Self::Intent,
        next: Self::Intent,
    ) -> Result<Self::Intent, DomainIssue> {
        let mut merged = previous.clone();
        if let (Some(target), Some(source)) = (merged.as_object_mut(), next.as_object()) {
            for (key, value) in source {
                target.insert(key.clone(), value.clone());
            }
        }
        Ok(merged)
    }

    fn apply(&self, intent: &Self::Intent, value: &mut Value) -> Result<(), DomainIssue> {
        if let (Some(target), Some(source)) = (value.as_object_mut(), intent.as_object()) {
            for (key, intent_value) in source {
                target.insert(key.clone(), intent_value.clone());
            }
        }
        Ok(())
    }

    fn pending_weight(&self, intent: &Self::Intent) -> usize {
        intent.to_string().len()
    }
}

struct ZeroClock;

impl DebounceClock for ZeroClock {
    fn now(&self) -> Duration {
        Duration::ZERO
    }
}

#[test]
fn debounced_flush_refuses_and_never_writes() {
    let fixture = Fixture::new();
    let original = "name = \"cli\"\nenabled = true\n";
    fixture.write_config("cli-settings.toml", original);
    let domain = SettingsDomain::new();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let policy = DebouncePolicy::new(Duration::from_millis(50), 256, mutation_options()).unwrap();
    let mut lane =
        DebouncedMutation::new(&store, &domain, MergeStrategy, ZeroClock, policy).unwrap();
    lane.stage(json!({"enabled": false})).unwrap();

    let outcome = lane.flush_forced();
    let longhorn_config::FlushOutcome::Terminal(terminal) = outcome else {
        panic!("expected a terminal flush outcome");
    };
    let longhorn_config::DebounceTerminal::Failed { error, .. } = terminal else {
        panic!("expected a failed flush, got {terminal:?}");
    };
    assert!(matches!(
        error,
        MutationError::Refused(MutationRefusal::HumanWrittenToml { .. })
    ));
    assert_eq!(
        fixture.read_config("cli-settings.toml"),
        original.as_bytes()
    );
}

// --- backup ----------------------------------------------------------------

fn capture_metadata(archive_id: &str) -> BackupMetadata {
    BackupMetadata::new(
        archive_id,
        BackupKind::Operational,
        "2026-10-22T12:00:00Z",
        BackupApplication::new("com.example.cli", "1.0.0").unwrap(),
        BackupProducer::new("longhorn-config", "0.1.0").unwrap(),
    )
    .unwrap()
}

#[test]
fn backup_records_a_toml_domain_as_an_explicit_exclusion() {
    let fixture = Fixture::new();
    let original = "name = \"cli\"\nenabled = true\n";
    fixture.write_config("cli-settings.toml", original);
    let domain = SettingsDomain::new();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let catalog = BackupCatalog::new();
    let snapshot = store
        .capture_backup(
            &catalog,
            &BackupScope::AllRegistered,
            capture_metadata("cli-backup-1"),
            BackupCaptureOptions::new(Duration::from_secs(2), BackupLimits::default()),
        )
        .unwrap();

    let exclusions: &[BackupExclusion] = snapshot.manifest().exclusions();
    assert_eq!(exclusions.len(), 1);
    assert_eq!(exclusions[0].domain(), domain.descriptor().id());
    assert_eq!(
        exclusions[0].reason(),
        "human-written, read-only in phase 1"
    );
    assert!(snapshot.manifest().domains().is_empty());
    assert_eq!(
        fixture.read_config("cli-settings.toml"),
        original.as_bytes()
    );
}

#[test]
fn backup_catalog_include_refuses_a_toml_domain() {
    let fixture = Fixture::new();
    fixture.write_config("cli-settings.toml", "name = \"cli\"\nenabled = true\n");
    let domain = SettingsDomain::new();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let mut catalog = BackupCatalog::new();
    catalog.include(&domain).unwrap();

    let error = store
        .capture_backup(
            &catalog,
            &BackupScope::AllRegistered,
            capture_metadata("cli-backup-2"),
            BackupCaptureOptions::new(Duration::from_secs(2), BackupLimits::default()),
        )
        .unwrap_err();
    assert_eq!(
        error,
        BackupCaptureError::HumanWrittenToml {
            domain: domain.descriptor().id().clone(),
        }
    );
}

#[test]
fn restore_inspection_refuses_an_ordinary_target_for_a_toml_domain() {
    // Donor: a JSON domain under the exact identity the target registers as
    // human TOML, captured as an ordinary present archive payload.
    let donor = Fixture::new();
    let donor_domain = JsonDomain::named("example.cli.settings", "cli-settings.json", 2);
    donor.write_config(
        "cli-settings.json",
        &json!({
            "domain": "example.cli.settings",
            "schemaVersion": 2,
            "value": {"name": "donor", "enabled": true},
        })
        .to_string(),
    );
    let mut donor_store = donor.store();
    donor_store.register(&donor_domain).unwrap();
    let mut donor_catalog = BackupCatalog::new();
    donor_catalog.include(&donor_domain).unwrap();
    let snapshot = donor_store
        .capture_backup(
            &donor_catalog,
            &BackupScope::AllRegistered,
            capture_metadata("cli-donor"),
            BackupCaptureOptions::new(Duration::from_secs(2), BackupLimits::default()),
        )
        .unwrap();
    let encoded = encode_backup_archive(&snapshot, BackupArchiveLimits::default()).unwrap();
    let archive = inspect_backup_archive(encoded.bytes(), BackupArchiveLimits::default()).unwrap();

    // Target: the same identity as a human TOML domain.
    let target = Fixture::new();
    let original = "name = \"cli\"\nenabled = true\n";
    target.write_config("cli-settings.toml", original);
    let toml_domain = SettingsDomain::new();
    let mut target_store = target.store();
    target_store.register(&toml_domain).unwrap();
    let mut target_catalog = BackupCatalog::new();
    // An include policy would be honoured for a JSON domain; the TOML
    // refusal must override it.
    target_catalog.include(&toml_domain).unwrap();

    let inspection = target_store.inspect_restore(
        &target_catalog,
        &archive,
        &BackupApplication::new("com.example.cli", "1.0.0").unwrap(),
        &BackupProducer::new("longhorn-config", "0.1.0").unwrap(),
    );

    let report = inspection
        .domains()
        .iter()
        .find(|domain| domain.domain() == toml_domain.descriptor().id())
        .expect("the manifest domain must be inspected against the target");
    assert_eq!(
        report.compatibility(),
        &RestoreDomainCompatibility::PolicyExcluded {
            reason: "human-written, read-only in phase 1".to_owned(),
        }
    );
    assert_eq!(inspection.receipt().restorable(), 0);
    assert_eq!(target.read_config("cli-settings.toml"), original.as_bytes());
}

// --- type mapping ----------------------------------------------------------

#[test]
fn type_mapping_covers_datetimes_large_integers_and_empty_tables() {
    let fixture = Fixture::new();
    fixture.write_config(
        "cli-freeform.toml",
        "stamp = 1979-05-27T07:32:00Z\n\
         big = 9223372036854775807\n\
         small = -9223372036854775808\n\
         hex = 0xff\n\
         [table]\n",
    );
    let domain = FreeformDomain::new("cli-freeform.toml");
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let LoadOutcome::Ready(loaded) = store.load(&domain).unwrap() else {
        panic!("expected a ready load");
    };
    assert_eq!(
        loaded.value,
        json!({
            "stamp": "1979-05-27T07:32:00Z",
            "big": 9223372036854775807i64,
            "small": -9223372036854775808i64,
            "hex": 255,
            "table": {},
        })
    );
}

#[test]
fn local_datetimes_are_refused_with_line_and_column() {
    let fixture = Fixture::new();
    fixture.write_config(
        "cli-freeform.toml",
        "ok = \"yes\"\nstamp = 1979-05-27T07:32:00\n",
    );
    let domain = FreeformDomain::new("cli-freeform.toml");
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
        panic!("expected a recovery state");
    };
    assert_eq!(recovery.kind, RecoveryKind::CorruptDocument);
    assert!(recovery.detail.contains("local date-time"));
    let position = recovery.position.expect("refusals carry a position");
    assert_eq!(position.line, 2);
}

#[test]
fn non_finite_floats_and_out_of_range_integers_are_refused() {
    for contents in [
        "value = nan\n",
        "value = inf\n",
        "value = 9223372036854775808\n",
    ] {
        let fixture = Fixture::new();
        fixture.write_config("cli-freeform.toml", contents);
        let domain = FreeformDomain::new("cli-freeform.toml");
        let mut store = fixture.store();
        store.register(&domain).unwrap();

        let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
            panic!("expected a recovery state for {contents:?}");
        };
        assert_eq!(recovery.kind, RecoveryKind::CorruptDocument, "{contents:?}");
        assert!(recovery.position.is_some(), "{contents:?}");
        assert_eq!(
            fs::read(fixture.config.join("cli-freeform.toml")).unwrap(),
            contents.as_bytes()
        );
    }
}

// --- JSON domains stay byte-identical ---------------------------------------

struct JsonDomain {
    descriptor: DomainDescriptor,
}

impl JsonDomain {
    fn new() -> Self {
        Self::named("example.cli.json", "cli-json.json", 1)
    }

    /// A JSON domain with an exact id, path, and schema version, so a donor
    /// archive can name a target store's domain.
    fn named(id: &str, path: &str, version: u32) -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new(id).unwrap(),
                SchemaVersion::new(version).unwrap(),
                USER_CONFIG_CLASS,
                Some(DomainFilePath::new(path).unwrap()),
            )
            .unwrap(),
        }
    }
}

impl ConfigDomain for JsonDomain {
    type Value = Value;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        json!({"enabled": false})
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        Ok(value)
    }

    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        Ok(value.clone())
    }

    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue> {
        if value.get("enabled").is_some_and(Value::is_boolean) {
            Ok(())
        } else {
            Err(DomainIssue::new("enabled", "enabled must be boolean"))
        }
    }

    fn validate_raw(
        &self,
        _schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        self.validate(value)
    }

    fn migrate_one(
        &self,
        _from: SchemaVersion,
        _value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        Ok(None)
    }
}

#[test]
fn json_domains_keep_their_envelope_bytes_and_behaviour() {
    let fixture = Fixture::new();
    let domain = JsonDomain::new();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    // First load: compiled default with no file on disk.
    let outcome = store.load(&domain).unwrap();
    let LoadOutcome::Ready(first) = outcome else {
        panic!("expected a ready JSON load");
    };
    assert_eq!(first.origin, LoadedOrigin::Default);

    // A mutation writes the versioned JSON envelope, pretty-printed.
    let receipt = store
        .mutate(&domain, mutation_options(), |value| {
            value["enabled"] = Value::Bool(true);
            Ok(())
        })
        .unwrap();
    let bytes = fs::read(&receipt.path).unwrap();
    let envelope: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        envelope,
        json!({
            "domain": "example.cli.json",
            "schemaVersion": 1,
            "value": {"enabled": true},
        })
    );
    assert_eq!(
        bytes,
        serde_json::to_vec_pretty(&envelope).unwrap(),
        "the envelope must stay serde_json's pretty format"
    );

    // The next load reads the published file.
    let outcome = store.load(&domain).unwrap();
    let LoadOutcome::Ready(second) = outcome else {
        panic!("expected a ready JSON reload");
    };
    assert_eq!(second.origin, LoadedOrigin::File);
    assert_eq!(second.value, json!({"enabled": true}));
}
