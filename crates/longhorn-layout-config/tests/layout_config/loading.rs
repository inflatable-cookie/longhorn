use longhorn_config::{
    ConfigStore, DomainIssue, LoadOutcome, LoadedOrigin, MigrationStep, RecoveryKind,
};
use longhorn_core::SchemaVersion;
use longhorn_layout_config::{
    LayoutBackupPolicy, LayoutMigration, LayoutMigrationTarget, NoLayoutMigration,
    PersistedLayoutDocument, RegisteredLayoutDomain, compute_layout_registry_digest,
};
use serde_json::Value;

use crate::support::{
    Fixture, current_value, descriptor, document, domain, envelope, registry,
    registry_with_maximum_schemas,
};

#[test]
fn missing_current_and_corrupt_sources_remain_distinct() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let LoadOutcome::Ready(missing) = store.load(&domain).unwrap() else {
        panic!("missing source should return the validated default");
    };
    assert_eq!(missing.origin, LoadedOrigin::Default);
    assert_eq!(missing.value, document());
    assert!(missing.source.is_none());

    store
        .mutate(
            &domain,
            crate::support::options(std::time::Duration::from_secs(1)),
            |_value| Ok(()),
        )
        .unwrap();
    let LoadOutcome::Ready(current) = store.load(&domain).unwrap() else {
        panic!("published source should load");
    };
    assert_eq!(current.origin, LoadedOrigin::File);
    assert!(current.source.is_some());

    fixture.write(&domain, b"{ definitely not json");
    let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
        panic!("corrupt source should enter recovery");
    };
    assert_eq!(recovery.kind, RecoveryKind::CorruptDocument);
    assert_eq!(
        recovery.source.as_ref().unwrap().bytes,
        b"{ definitely not json"
    );
}

#[test]
fn future_schema_and_registry_mismatch_preserve_exact_source() {
    let fixture = Fixture::new();
    let domain = domain();
    let mut store = fixture.store();
    store.register(&domain).unwrap();

    let future = envelope("layout.workspace", 2, current_value(&domain));
    fixture.write(&domain, &future);
    let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
        panic!("future schema should enter recovery");
    };
    assert_eq!(recovery.kind, RecoveryKind::FutureSchema);
    assert_eq!(recovery.source.unwrap().bytes, future);

    let changed = RegisteredLayoutDomain::new(
        descriptor(1),
        document(),
        registry_with_maximum_schemas(9),
        NoLayoutMigration,
        LayoutBackupPolicy::Include,
    )
    .unwrap();
    let mismatch = envelope("layout.workspace", 1, current_value(&domain));
    fixture.write(&changed, &mismatch);
    let mut changed_store = fixture.store();
    changed_store.register(&changed).unwrap();
    let LoadOutcome::Recovery(recovery) = changed_store.load(&changed).unwrap() else {
        panic!("registry mismatch should enter recovery");
    };
    assert_eq!(recovery.kind, RecoveryKind::InvalidValue);
    assert!(recovery.detail.contains("layout-registry-mismatch"));
    assert_eq!(recovery.source.unwrap().bytes, mismatch);
}

#[derive(Clone, Copy, Debug)]
struct RegistryMigration;

impl LayoutMigration for RegistryMigration {
    fn validate_raw(
        &self,
        schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        if schema_version.get() != 1 {
            return Err(DomainIssue::new("old-layout-schema", "expected schema 1"));
        }
        serde_json::from_value::<PersistedLayoutDocument>(value.clone())
            .map(|_| ())
            .map_err(|error| DomainIssue::new("old-layout-shape", error.to_string()))
    }

    fn migrate_one(
        &self,
        from: SchemaVersion,
        value: Value,
        target: LayoutMigrationTarget<'_>,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        if from.get() != 1 {
            return Ok(None);
        }
        let old: PersistedLayoutDocument = serde_json::from_value(value)
            .map_err(|error| DomainIssue::new("old-layout-decode", error.to_string()))?;
        Ok(Some(MigrationStep {
            schema_version: target.schema_version(),
            value: target.encode_current(old.document().clone())?,
        }))
    }
}

#[test]
fn registry_change_loads_only_through_explicit_schema_migration() {
    let fixture = Fixture::new();
    let old = domain();
    let old_bytes = envelope("layout.workspace", 1, current_value(&old));
    let current_registry = registry_with_maximum_schemas(9);
    let current = RegisteredLayoutDomain::new(
        descriptor(2),
        document(),
        current_registry,
        RegistryMigration,
        LayoutBackupPolicy::Include,
    )
    .unwrap();
    fixture.write(&current, &old_bytes);
    let mut store = fixture.store();
    store.register(&current).unwrap();

    let LoadOutcome::Ready(loaded) = store.load(&current).unwrap() else {
        panic!("explicit registry migration should load in memory");
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
}

#[test]
fn registry_digest_is_canonical_and_policy_sensitive() {
    let first = registry();
    let same = registry();
    let changed = registry_with_maximum_schemas(9);

    assert_eq!(
        compute_layout_registry_digest(&first).unwrap(),
        compute_layout_registry_digest(&same).unwrap()
    );
    assert_ne!(
        compute_layout_registry_digest(&first).unwrap(),
        compute_layout_registry_digest(&changed).unwrap()
    );
}

#[test]
fn missing_old_migration_is_inspectable() {
    let fixture = Fixture::new();
    let old = domain();
    let current = RegisteredLayoutDomain::new(
        descriptor(2),
        document(),
        registry(),
        NoLayoutMigration,
        LayoutBackupPolicy::Include,
    )
    .unwrap();
    let bytes = envelope("layout.workspace", 1, current_value(&old));
    fixture.write(&current, &bytes);
    let mut store = ConfigStore::new(fixture.roots(), fixture.coordination());
    store.register(&current).unwrap();

    let LoadOutcome::Recovery(recovery) = store.load(&current).unwrap() else {
        panic!("missing migration should enter recovery");
    };
    assert_eq!(recovery.kind, RecoveryKind::MissingMigration);
    assert_eq!(recovery.source.unwrap().bytes, bytes);
}
