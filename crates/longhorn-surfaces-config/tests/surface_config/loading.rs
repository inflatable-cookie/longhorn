use longhorn_config::{
    ConfigDomain, DomainIssue, LoadOutcome, LoadedOrigin, MigrationStep, RecoveryKind,
};
use longhorn_core::SchemaVersion;
use longhorn_surfaces_config::{
    NoSurfaceMigration, RegisteredSurfaceDomain, SurfaceBackupPolicy, SurfaceMigration,
    SurfaceMigrationTarget,
};
use serde_json::{Value, json};

use crate::support::{Fixture, descriptor, document, domain, envelope, limits};

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
        target: SurfaceMigrationTarget,
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
        limits(),
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
        limits(),
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
