use std::time::Duration;

use longhorn_config::{
    BackupApplication, BackupCaptureOptions, BackupCatalog, BackupExclusionReason, BackupKind,
    BackupLimits, BackupMetadata, BackupProducer, BackupScope, DomainDescriptor, DomainFilePath,
    StorageClass,
};
use longhorn_core::{DomainId, SchemaVersion};
use longhorn_surfaces_config::{LayoutBackupPolicy, NoLayoutMigration, RegisteredLayoutDomain};

use crate::support::{Fixture, document, domain, registry};

#[test]
fn backup_catalogue_uses_each_layout_domains_explicit_policy() {
    let fixture = Fixture::new();
    let included = domain();
    let excluded = RegisteredLayoutDomain::new(
        DomainDescriptor::new(
            DomainId::new("layout.session").unwrap(),
            SchemaVersion::new(1).unwrap(),
            StorageClass::MachineState,
            Some(DomainFilePath::new("session/layout.json").unwrap()),
        )
        .unwrap(),
        document(),
        registry(),
        NoLayoutMigration,
        LayoutBackupPolicy::Exclude(
            BackupExclusionReason::new("session-layout-is-recreatable").unwrap(),
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
                "layout-policy-fixture",
                BackupKind::Operational,
                "2026-07-28T12:00:00Z",
                BackupApplication::new("test.longhorn", "0.1.0").unwrap(),
                BackupProducer::new("longhorn-surfaces-config", "0.1.0").unwrap(),
            )
            .unwrap(),
            BackupCaptureOptions::new(Duration::from_secs(1), BackupLimits::default()),
        )
        .unwrap();

    assert_eq!(snapshot.manifest().domains().len(), 1);
    assert_eq!(
        snapshot.manifest().domains()[0].domain(),
        included.descriptor().id()
    );
    assert_eq!(snapshot.manifest().exclusions().len(), 1);
    assert_eq!(
        snapshot.manifest().exclusions()[0].domain(),
        excluded.descriptor().id()
    );
    assert_eq!(
        snapshot.manifest().exclusions()[0].reason(),
        "session-layout-is-recreatable"
    );
}
