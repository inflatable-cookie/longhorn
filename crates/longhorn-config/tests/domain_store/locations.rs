use longhorn_config::{
    ConfigDomain, ConfigStore, DomainLocation, LoadOutcome, LoadedOrigin, RegistrationError,
    RootKind, StorageClass, StorageRoots, StoreError, UnavailableState,
};
use longhorn_core::DomainId;

use super::common::{Fixture, PreferencesDomain, config_domain};

#[test]
fn storage_classes_resolve_to_distinct_authorities() {
    let fixture = Fixture::new();
    let cases = [
        (StorageClass::UserConfig, RootKind::Config),
        (StorageClass::MachineState, RootKind::State),
        (StorageClass::WorkspaceLocal, RootKind::Workspace),
        (StorageClass::ProjectShared, RootKind::Project),
        (StorageClass::Cache, RootKind::Cache),
    ];

    for (index, (class, expected_root)) in cases.into_iter().enumerate() {
        let domain = PreferencesDomain::new(
            &format!("example.domain{index}"),
            class,
            Some(&format!("example/domain-{index}.json")),
            1,
        );

        match fixture.roots.resolve(domain.descriptor()) {
            DomainLocation::File(file) => assert_eq!(file.root_kind(), expected_root),
            location => panic!("expected file location, found {location:?}"),
        }
    }

    let defaults = PreferencesDomain::new("example.defaults", StorageClass::Defaults, None, 1);
    let secret = PreferencesDomain::new("example.secret", StorageClass::Secret, None, 1);
    assert_eq!(
        fixture.roots.resolve(defaults.descriptor()),
        DomainLocation::DefaultsOnly
    );
    assert_eq!(
        fixture.roots.resolve(secret.descriptor()),
        DomainLocation::SecureStoreRequired
    );

    let roots_without_project = StorageRoots::new(
        fixture.temp.path().join("plain-config"),
        fixture.temp.path().join("plain-data"),
        fixture.temp.path().join("plain-state"),
        fixture.temp.path().join("plain-cache"),
        fixture.temp.path().join("plain-runtime"),
        fixture.temp.path().join("plain-log"),
        fixture.temp.path().join("plain-backups"),
    )
    .unwrap();
    let project = PreferencesDomain::new(
        "example.project",
        StorageClass::ProjectShared,
        Some("example/project.json"),
        1,
    );
    assert!(matches!(
        roots_without_project.resolve(project.descriptor()),
        DomainLocation::RootRequired {
            root: RootKind::Project,
            ..
        }
    ));
}

#[test]
fn registration_rejects_duplicate_ids_and_locations() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let first = config_domain();
    let duplicate_id = config_domain();
    let duplicate_path = PreferencesDomain::new(
        "example.other",
        StorageClass::UserConfig,
        Some("example/preferences.json"),
        3,
    );

    store.register(&first).unwrap();
    assert_eq!(
        store.register(&duplicate_id),
        Err(RegistrationError::DuplicateDomainId {
            id: DomainId::new("example.preferences").unwrap()
        })
    );
    assert_eq!(
        store.register(&duplicate_path),
        Err(RegistrationError::DuplicateLocation {
            existing: DomainId::new("example.preferences").unwrap(),
            incoming: DomainId::new("example.other").unwrap(),
        })
    );
}

#[test]
fn unregistered_domains_cannot_be_loaded() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let domain = config_domain();

    assert_eq!(
        store.load(&domain),
        Err(StoreError::NotRegistered {
            id: DomainId::new("example.preferences").unwrap()
        })
    );
}

#[test]
fn defaults_and_external_authorities_do_not_become_files() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let defaults =
        PreferencesDomain::new("example.compiled-defaults", StorageClass::Defaults, None, 3);
    let secret = PreferencesDomain::new("example.credentials", StorageClass::Secret, None, 3);
    store.register(&defaults).unwrap();
    store.register(&secret).unwrap();

    let LoadOutcome::Ready(loaded) = store.load(&defaults).unwrap() else {
        panic!("expected compiled default");
    };
    assert_eq!(loaded.origin, LoadedOrigin::Default);
    assert!(loaded.diagnostics.is_empty());

    let LoadOutcome::Unavailable(unavailable) = store.load(&secret).unwrap() else {
        panic!("expected secure-store requirement");
    };
    assert_eq!(
        unavailable,
        UnavailableState::Authority {
            location: DomainLocation::SecureStoreRequired
        }
    );

    let roots_without_project = StorageRoots::new(
        fixture.temp.path().join("isolated-config"),
        fixture.temp.path().join("isolated-data"),
        fixture.temp.path().join("isolated-state"),
        fixture.temp.path().join("isolated-cache"),
        fixture.temp.path().join("isolated-runtime"),
        fixture.temp.path().join("isolated-log"),
        fixture.temp.path().join("isolated-backups"),
    )
    .unwrap();
    let mut store = ConfigStore::new(roots_without_project, fixture.coordination.clone());
    let project = PreferencesDomain::new(
        "example.project-settings",
        StorageClass::ProjectShared,
        Some("example/project.json"),
        3,
    );
    store.register(&project).unwrap();
    assert!(matches!(
        store.load(&project).unwrap(),
        LoadOutcome::Unavailable(_)
    ));
}
