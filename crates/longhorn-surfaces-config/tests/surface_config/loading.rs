use std::{fs, time::Duration};

use longhorn_config::{
    BackupApplication, BackupArchiveFileName, BackupArchiveLimits, BackupCaptureOptions,
    BackupCatalog, BackupKind, BackupLimits, BackupMetadata, BackupOperationalRoot, BackupProducer,
    ConfigDomain, DomainIssue, LoadOutcome, LoadedOrigin, MigrationRewriteOptions, MigrationStep,
    RecoveryKind, RestoreSafetyBackupOptions, Sha256Digest, inspect_backup_archive,
};
use longhorn_core::{LayoutSchemaId, SchemaVersion, SurfaceRevision};
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutSchemaDefinition, PanelDefinition,
    ParticipatingWindow, RegionDefinition, SurfaceDocument, SurfaceValidationCode,
};
use longhorn_surfaces_config::{
    LayoutRegistryDigest, LegacySurfaceFormatMigration, NoSurfaceMigration,
    RegisteredSurfaceDomain, RegisteredSurfaceDomainError, SurfaceBackupPolicy, SurfaceMigration,
    SurfaceMigrationTarget,
};
use serde::Serialize;
use serde_json::{Value, json};

use crate::support::{
    Fixture, descriptor, document, domain, envelope, limits, registry, surface, surface_id,
    window_id,
};

/// The compiled default is the arrangement a consumer ships. Re-sorting it would
/// make the registered value differ from the source, so it is refused instead.
#[test]
fn a_non_canonical_default_is_refused_rather_than_resorted() {
    let unsorted = SurfaceDocument::new(
        SurfaceRevision::new(7),
        [surface("surface:b", "B", 1), surface("surface:a", "A", 0)],
        [],
        [ParticipatingWindow::new(
            window_id("window:main"),
            Some(surface_id("surface:a")),
        )],
    );

    let error = RegisteredSurfaceDomain::new(
        descriptor(1),
        unsorted,
        registry(),
        NoSurfaceMigration,
        SurfaceBackupPolicy::Include,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        RegisteredSurfaceDomainError::InvalidDefault {
            code: SurfaceValidationCode::NonCanonicalDocument,
            ..
        }
    ));
}

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
        LegacySurfaceFormatMigration::digestless(SchemaVersion::new(1).unwrap()),
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

    // A digest-bearing file offered to the digest-less migration fails closed into recovery.
    let digest_bearing = envelope(
        "surfaces.workspace",
        1,
        json!({
            "registryDigest": AUTHENTIC_AA38E725_LAYOUT_DIGEST,
            "document": document(),
        }),
    );
    fixture.write(&upgraded, &digest_bearing);
    let LoadOutcome::Recovery(recovery) = store.load(&upgraded).unwrap() else {
        panic!("digest-bearing file must be rejected by digestless migration");
    };
    assert_eq!(recovery.kind, RecoveryKind::InvalidValue);
    assert_eq!(recovery.source.unwrap().bytes, digest_bearing);
}

// Authentic SHA-256 produced by the pre-unification (`aa38e725`)
// `compute_layout_registry_digest` over `support::registry()`'s 7-field
// `LayoutLimits` + `schemas` + `panelDefinitions` in struct declaration order.
const AUTHENTIC_AA38E725_LAYOUT_DIGEST: &str =
    "23ca9d33b2b0f0aad7e4f41b9b1727eb5872608bb1c5398334404f7ef5ea7adb";

#[test]
fn legacy_layout_file_with_expected_previous_digest_migrates_and_rejects_mismatched_digests() {
    let fixture = Fixture::new();
    let current_registry = registry();
    assert_eq!(
        legacy_aa38e725_registry_digest(&current_registry).as_str(),
        AUTHENTIC_AA38E725_LAYOUT_DIGEST
    );
    let expected_previous_digest =
        LayoutRegistryDigest::new(AUTHENTIC_AA38E725_LAYOUT_DIGEST).unwrap();
    let legacy_bytes = envelope(
        "surfaces.workspace",
        1,
        json!({
            "registryDigest": AUTHENTIC_AA38E725_LAYOUT_DIGEST,
            "document": document(),
        }),
    );
    let upgraded = RegisteredSurfaceDomain::new(
        descriptor(2),
        document(),
        current_registry,
        LegacySurfaceFormatMigration::with_expected_digest(
            SchemaVersion::new(1).unwrap(),
            expected_previous_digest.clone(),
        ),
        SurfaceBackupPolicy::Include,
    )
    .unwrap();
    assert_ne!(upgraded.registry_digest(), &expected_previous_digest);

    let mut store = fixture.store();
    store.register(&upgraded).unwrap();

    // An all-zero or otherwise unexpected source digest must fail closed into recovery.
    let bogus_digest_bytes = envelope(
        "surfaces.workspace",
        1,
        json!({
            "registryDigest": "0".repeat(64),
            "document": document(),
        }),
    );
    fixture.write(&upgraded, &bogus_digest_bytes);
    let LoadOutcome::Recovery(recovery) = store.load(&upgraded).unwrap() else {
        panic!("unexpected source digest must enter recovery instead of being replaced");
    };
    assert_eq!(recovery.kind, RecoveryKind::InvalidValue);
    assert_eq!(recovery.source.unwrap().bytes, bogus_digest_bytes);
    assert_eq!(
        fs::read(fixture.path(&upgraded)).unwrap(),
        bogus_digest_bytes
    );

    // A digest-less file offered to the digest-bearing migration must also enter recovery.
    let digestless_bytes = envelope("surfaces.workspace", 1, json!({ "document": document() }));
    fixture.write(&upgraded, &digestless_bytes);
    let LoadOutcome::Recovery(recovery) = store.load(&upgraded).unwrap() else {
        panic!("digest-less file must be rejected by digest-bearing migration");
    };
    assert_eq!(recovery.kind, RecoveryKind::InvalidValue);
    assert_eq!(recovery.source.unwrap().bytes, digestless_bytes);

    // A file carrying the exact expected previous digest migrates in memory and rewrites safely.
    fixture.write(&upgraded, &legacy_bytes);
    let LoadOutcome::Ready(loaded) = store.load(&upgraded).unwrap() else {
        panic!("legacy layout file with expected prior digest should migrate in memory");
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
        LegacySurfaceFormatMigration::digestless(SchemaVersion::new(1).unwrap()),
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

#[derive(Serialize)]
struct LegacyAa38e725LayoutLimits {
    maximum_schemas: usize,
    maximum_regions_per_schema: usize,
    maximum_sizing_slots_per_schema: usize,
    maximum_panel_definitions: usize,
    maximum_surfaces: usize,
    maximum_panel_instances: usize,
    maximum_panel_instances_per_region: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyAa38e725RegistryDigestMaterial<'registry> {
    limits: LegacyAa38e725LayoutLimits,
    schemas: Vec<&'registry LayoutSchemaDefinition>,
    panel_definitions: Vec<&'registry PanelDefinition>,
}

fn legacy_aa38e725_registry_digest(registry: &LayoutDefinitionRegistry) -> LayoutRegistryDigest {
    let limits = registry.limits();
    let material = LegacyAa38e725RegistryDigestMaterial {
        limits: LegacyAa38e725LayoutLimits {
            maximum_schemas: limits.maximum_schemas(),
            maximum_regions_per_schema: limits.maximum_regions_per_schema(),
            maximum_sizing_slots_per_schema: limits.maximum_sizing_slots_per_schema(),
            maximum_panel_definitions: limits.maximum_panel_definitions(),
            maximum_surfaces: limits.maximum_surfaces(),
            maximum_panel_instances: limits.maximum_panel_instances(),
            maximum_panel_instances_per_region: limits.maximum_panel_instances_per_region(),
        },
        schemas: registry.schemas().collect(),
        panel_definitions: registry.panel_definitions().collect(),
    };
    let bytes = serde_json::to_vec(&material).unwrap();
    LayoutRegistryDigest::from_sha256(Sha256Digest::from_bytes(&bytes))
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
