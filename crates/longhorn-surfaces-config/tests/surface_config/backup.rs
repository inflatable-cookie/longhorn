use longhorn_config::{
    BackupApplication, BackupCaptureOptions, BackupCatalog, BackupExclusionReason, BackupKind,
    BackupLimits, BackupMetadata, BackupProducer, BackupScope, DomainDescriptor, DomainFilePath,
    StorageClass,
};
use longhorn_core::{DomainId, SchemaVersion};
use longhorn_surfaces_config::{NoSurfaceMigration, RegisteredSurfaceDomain, SurfaceBackupPolicy};

use crate::support::{Fixture, document, domain, limits};

#[test]
fn backup_catalogue_preserves_include_and_exclude_policy() {
    let fixture = Fixture::new();
    let included = domain();
    let excluded = RegisteredSurfaceDomain::new(
        DomainDescriptor::new(
            DomainId::new("surfaces.session").unwrap(),
            SchemaVersion::new(1).unwrap(),
            StorageClass::MachineState,
            Some(DomainFilePath::new("session/surfaces.json").unwrap()),
        )
        .unwrap(),
        document(),
        limits(),
        NoSurfaceMigration,
        SurfaceBackupPolicy::Exclude(
            BackupExclusionReason::new("session-surfaces-are-recreatable").unwrap(),
        ),
    )
    .unwrap();
    let mut store = fixture.store();
    store.register(&included).unwrap();
    store.register(&excluded).unwrap();
    let mut catalog = BackupCatalog::new();
    included.add_to_backup_catalog(&mut catalog).unwrap();
    excluded.add_to_backup_catalog(&mut catalog).unwrap();

    let snapshot = store
        .capture_backup(
            &catalog,
            &BackupScope::AllRegistered,
            BackupMetadata::new(
                "surface-policy-fixture",
                BackupKind::Operational,
                "2026-07-29T12:00:00Z",
                BackupApplication::new("test.longhorn", "0.1.0").unwrap(),
                BackupProducer::new("longhorn-surfaces-config", "0.1.0").unwrap(),
            )
            .unwrap(),
            BackupCaptureOptions::new(std::time::Duration::from_secs(1), BackupLimits::default()),
        )
        .unwrap();
    assert_eq!(snapshot.manifest().domains().len(), 1);
    assert_eq!(snapshot.manifest().exclusions().len(), 1);
    assert_eq!(
        snapshot.manifest().exclusions()[0].reason(),
        "session-surfaces-are-recreatable"
    );
}
