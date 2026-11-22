use std::{fs, time::Duration};

use longhorn_config::{
    BackupApplication, BackupArchiveFileName, BackupArchiveLimits, BackupCaptureOptions,
    BackupCatalog, BackupKind, BackupLimits, BackupMetadata, BackupOperationalRoot, BackupProducer,
    ConfigDomain, DomainIssue, LoadOutcome, LoadedOrigin, MigrationRewriteOptions, MigrationStep,
    RecoveryKind, RestoreSafetyBackupOptions, inspect_backup_archive,
};
use longhorn_core::{LayoutSchemaId, SchemaVersion};
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutSchemaDefinition, RegionDefinition,
};
use longhorn_surfaces_config::{
    LegacySurfaceFormatMigration, NoSurfaceMigration, RegisteredSurfaceDomain, SurfaceBackupPolicy,
    SurfaceMigration, SurfaceMigrationTarget,
};
use serde_json::{Value, json};

use crate::support::{Fixture, descriptor, document, domain, envelope, limits, registry};

#[test]
fn missing_corrupt_future_and_incompatible_sources_remain_distinct() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let LoadOutcome::Ready(missing) = store.load(&domain).unwrap() else {
        panic!("missing source should return default");
    };
    assert_eq!(missing.origin, LoadedOrigin::Default);
    assert_eq!(missing.value, document());

    fixture.write(&domain, b"{ not json");
    let LoadOutcome::Recovery(corrupt) = store.load(&domain).unwrap() else {
        panic!("corrupt source should enter recovery");
    };
    assert_eq!(corrupt.kind, RecoveryKind::CorruptDocument);
    assert_eq!(corrupt.source.unwrap().bytes, b"{ not json");

    let current = domain.encode(&document()).unwrap();
    let future = envelope("surfaces.workspace", 2, current);
    fixture.write(&domain, &future);
    let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
        panic!("future source should enter recovery");
    };
    assert_eq!(recovery.kind, RecoveryKind::FutureSchema);
    assert_eq!(recovery.source.unwrap().bytes, future);

    let incompatible = envelope(
        "surfaces.workspace",
        1,
        json!({"document": {"revision": 7, "surfaces": [], "windows": [
            {"id": "window:main", "active_surface_id": "surface:missing"}
        ]}}),
    );
    fixture.write(&domain, &incompatible);
    let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
        panic!("incompatible current source should enter recovery");
    };
    assert_eq!(recovery.kind, RecoveryKind::InvalidValue);
    assert_eq!(recovery.source.unwrap().bytes, incompatible);
}

#[derive(Clone, Copy, Debug)]
struct OldShapeMigration;

impl SurfaceMigration for OldShapeMigration {
    fn validate_raw(
        &self,
        schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        if schema_version.get() != 1 || value.get("surfaceDocument").is_none() {
            return Err(DomainIssue::new(
                "old-surface-shape",
                "expected schema 1 wrapper",
            ));
        }
        Ok(())
    }

    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
        target: SurfaceMigrationTarget<'_>,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        if from.get() != 1 {
            return Ok(None);
        }
        let document = serde_json::from_value(
            value
                .get("surfaceDocument")
                .cloned()
                .ok_or_else(|| DomainIssue::new("old-surface-shape", "missing document"))?,
        )
        .map_err(|error| DomainIssue::new("old-surface-decode", error.to_string()))?;
        Ok(Some(MigrationStep {
            schema_version: target.schema_version(),
            value: target.encode_current(document)?,
        }))
    }
}

#[test]
fn document_shape_change_requires_explicit_schema_migration() {
    let fixture = Fixture::new();
    let old_value = json!({"surfaceDocument": document()});
    let old_bytes = envelope("surfaces.workspace", 1, old_value.clone());
    let current = RegisteredSurfaceDomain::new(
        descriptor(2),
        document(),
        registry(),
        OldShapeMigration,
        SurfaceBackupPolicy::Include,
    )
    .unwrap();
    fixture.write(&current, &old_bytes);
    let mut store = fixture.store();
    store.register(&current).unwrap();
    let LoadOutcome::Ready(loaded) = store.load(&current).unwrap() else {
        panic!("explicit shape migration should load");
    };
    assert_eq!(
        loaded.origin,
        LoadedOrigin::MigratedInMemory {
            from: SchemaVersion::new(1).unwrap(),
            to: SchemaVersion::new(2).unwrap(),
        }
    );
    assert_eq!(loaded.value, document());
    assert_eq!(std::fs::read(fixture.path(&current)).unwrap(), old_bytes);

    let missing = RegisteredSurfaceDomain::new(
        descriptor(2),
        document(),
        registry(),
        NoSurfaceMigration,
        SurfaceBackupPolicy::Include,
    )
    .unwrap();
    fixture.write(&missing, &envelope("surfaces.workspace", 1, old_value));
    let mut missing_store = fixture.store();
    missing_store.register(&missing).unwrap();
    let LoadOutcome::Recovery(recovery) = missing_store.load(&missing).unwrap() else {
        panic!("missing migration should enter recovery");
    };
    assert_eq!(recovery.kind, RecoveryKind::MissingMigration);
}

#[test]
fn legacy_digestless_surface_file_migrates_in_memory_and_rewrites_with_safety_backup() {
    let fixture = Fixture::new();
    // Exact raw shape written by the pre-unification PersistedSurfaceDocument (no registryDigest).
    let legacy_bytes = envelope("surfaces.workspace", 1, json!({ "document": document() }));
    let upgraded = RegisteredSurfaceDomain::new(
        descriptor(2),
        document(),
        registry(),
        LegacySurfaceFormatMigration::new(SchemaVersion::new(1).unwrap()),
        SurfaceBackupPolicy::Include,
    )
    .unwrap();
    fixture.write(&upgraded, &legacy_bytes);
    let mut store = fixture.store();
    store.register(&upgraded).unwrap();

    let LoadOutcome::Ready(loaded) = store.load(&upgraded).unwrap() else {
        panic!("legacy digest-less surface file should migrate in memory");
    };
    assert_eq!(
        loaded.origin,
        LoadedOrigin::MigratedInMemory {
            from: SchemaVersion::new(1).unwrap(),
            to: SchemaVersion::new(2).unwrap(),
        }
    );
    assert_eq!(loaded.value, document());
    assert_eq!(fs::read(fixture.path(&upgraded)).unwrap(), legacy_bytes);

    let mut catalog = BackupCatalog::new();
    upgraded.add_to_backup_catalog(&mut catalog).unwrap();
    let receipt = store
        .rewrite_migrated_domain(
            &catalog,
            &upgraded,
            MigrationRewriteOptions::new(
                Duration::from_secs(2),
                safety_options(&fixture, "pre-migration-surface"),
            ),
        )
        .unwrap();
    assert_eq!(receipt.from().get(), 1);
    assert_eq!(receipt.to().get(), 2);

    let safety_bytes = fs::read(&receipt.safety_backup().path).unwrap();
    let safety_archive =
        inspect_backup_archive(&safety_bytes, BackupArchiveLimits::default()).unwrap();
    assert_eq!(safety_archive.payloads()[0].bytes(), legacy_bytes);

    let rewritten: Value =
        serde_json::from_slice(&fs::read(fixture.path(&upgraded)).unwrap()).unwrap();
    assert_eq!(rewritten["schemaVersion"], 2);
    assert_eq!(
        rewritten["value"]["registryDigest"],
        upgraded.registry_digest().as_str()
    );

    let LoadOutcome::Ready(reloaded) = store.load(&upgraded).unwrap() else {
        panic!("rewritten surface file should load directly");
    };
    assert_eq!(reloaded.origin, LoadedOrigin::File);
    assert_eq!(reloaded.value, document());
}

#[test]
fn legacy_layout_file_with_previous_digest_migrates_and_rewrites_with_safety_backup() {
    let fixture = Fixture::new();
    // Exact raw shape written by the pre-unification PersistedLayoutDocument (digest over old 7-field LayoutLimits).
    let previous_digest = "0".repeat(64);
    let legacy_bytes = envelope(
        "surfaces.workspace",
        1,
        json!({
            "registryDigest": previous_digest,
            "document": document(),
        }),
    );
    let upgraded = RegisteredSurfaceDomain::new(
        descriptor(2),
        document(),
        registry(),
        LegacySurfaceFormatMigration::new(SchemaVersion::new(1).unwrap()),
        SurfaceBackupPolicy::Include,
    )
    .unwrap();
    fixture.write(&upgraded, &legacy_bytes);
    let mut store = fixture.store();
    store.register(&upgraded).unwrap();

    let LoadOutcome::Ready(loaded) = store.load(&upgraded).unwrap() else {
        panic!("legacy layout file with prior digest should migrate in memory");
    };
    assert_eq!(
        loaded.origin,
        LoadedOrigin::MigratedInMemory {
            from: SchemaVersion::new(1).unwrap(),
            to: SchemaVersion::new(2).unwrap(),
        }
    );
    assert_eq!(loaded.value, document());
    assert_eq!(fs::read(fixture.path(&upgraded)).unwrap(), legacy_bytes);

    let mut catalog = BackupCatalog::new();
    upgraded.add_to_backup_catalog(&mut catalog).unwrap();
    let receipt = store
        .rewrite_migrated_domain(
            &catalog,
            &upgraded,
            MigrationRewriteOptions::new(
                Duration::from_secs(2),
                safety_options(&fixture, "pre-migration-layout"),
            ),
        )
        .unwrap();

    let safety_bytes = fs::read(&receipt.safety_backup().path).unwrap();
    let safety_archive =
        inspect_backup_archive(&safety_bytes, BackupArchiveLimits::default()).unwrap();
    assert_eq!(safety_archive.payloads()[0].bytes(), legacy_bytes);

    let LoadOutcome::Ready(reloaded) = store.load(&upgraded).unwrap() else {
        panic!("rewritten layout file should load directly");
    };
    assert_eq!(reloaded.origin, LoadedOrigin::File);
    assert_eq!(reloaded.value, document());
}

#[test]
fn legacy_file_requiring_product_choice_enters_recovery_with_original_bytes_preserved() {
    let fixture = Fixture::new();
    // A legacy document referencing `schema:test` cannot be auto-migrated into a
    // registry that only declares `schema:renamed` without a consumer product choice.
    let legacy_bytes = envelope("surfaces.workspace", 1, json!({ "document": document() }));
    let renamed_registry = LayoutDefinitionRegistry::new(
        limits(),
        [LayoutSchemaDefinition::new(
            LayoutSchemaId::new("schema:renamed").unwrap(),
            [RegionDefinition::new(
                longhorn_core::RegionId::new("region:main").unwrap(),
                longhorn_core::RegionFamilyId::new("family:main").unwrap(),
                0,
                EmptyRegionPolicy::KeepVisible,
                false,
            )],
            [],
        )],
        [],
    )
    .unwrap();
    let renamed_default =
        longhorn_surfaces::SurfaceDocument::new(longhorn_core::SurfaceRevision::new(1), [], [], []);
    let upgraded = RegisteredSurfaceDomain::new(
        descriptor(2),
        renamed_default,
        renamed_registry,
        LegacySurfaceFormatMigration::new(SchemaVersion::new(1).unwrap()),
        SurfaceBackupPolicy::Include,
    )
    .unwrap();
    fixture.write(&upgraded, &legacy_bytes);
    let mut store = fixture.store();
    store.register(&upgraded).unwrap();

    let LoadOutcome::Recovery(recovery) = store.load(&upgraded).unwrap() else {
        panic!("legacy file needing a product choice should enter recovery");
    };
    assert_eq!(recovery.kind, RecoveryKind::MigrationFailed);
    assert_eq!(recovery.source.unwrap().bytes, legacy_bytes);
    assert_eq!(fs::read(fixture.path(&upgraded)).unwrap(), legacy_bytes);
}

fn safety_options(fixture: &Fixture, archive_id: &str) -> RestoreSafetyBackupOptions {
    RestoreSafetyBackupOptions::new(
        BackupMetadata::new(
            archive_id,
            BackupKind::PreMigration,
            "2026-11-22T15:00:00Z",
            BackupApplication::new("com.example.surfaces", "1.0.0").unwrap(),
            BackupProducer::new("longhorn-surfaces-config", "0.1.0").unwrap(),
        )
        .unwrap(),
        BackupOperationalRoot::new(fixture.roots().backup().to_path_buf()).unwrap(),
        BackupArchiveFileName::new(format!("{archive_id}.longhorn-backup")).unwrap(),
        BackupCaptureOptions::new(Duration::from_secs(2), BackupLimits::default()),
        BackupArchiveLimits::default(),
    )
}
