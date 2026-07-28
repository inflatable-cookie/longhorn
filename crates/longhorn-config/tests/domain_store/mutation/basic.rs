use std::{fs, time::Duration};

use longhorn_config::{
    ConfigDomain, CoordinationFailureKind, DomainIssue, Durability, DurabilityRequirement,
    LoadOutcome, MutationError, MutationOptions, MutationRefusal, RecoveryKind, StorageClass,
};
use serde_json::{Value, json};

use crate::common::{Fixture, PreferencesDomain, config_domain, document};

fn options() -> MutationOptions {
    MutationOptions::new(Duration::from_secs(1), DurabilityRequirement::Atomic)
}

#[test]
fn missing_and_current_values_publish_the_registered_envelope() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = config_domain();
    store.register(&domain).unwrap();

    let receipt = store
        .mutate(&domain, options(), |value| {
            value.name = "first".to_owned();
            Ok(())
        })
        .unwrap();

    assert_eq!(receipt.domain, *domain.descriptor().id());
    assert_eq!(receipt.schema_version, domain.descriptor().schema_version());
    assert!(matches!(
        receipt.durability,
        Durability::FileSynced | Durability::FileAndDirectorySynced
    ));
    let serialized: Value = serde_json::from_slice(&fs::read(&receipt.path).unwrap()).unwrap();
    assert_eq!(
        serialized,
        json!({
            "domain": "example.preferences",
            "schemaVersion": 3,
            "value": {"name": "first", "enabled": true}
        })
    );

    store
        .mutate(&domain, options(), |value| {
            value.enabled = false;
            Ok(())
        })
        .unwrap();
    let LoadOutcome::Ready(loaded) = store.load(&domain).unwrap() else {
        panic!("expected current value");
    };
    assert_eq!(loaded.value.name, "first");
    assert!(!loaded.value.enabled);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        assert_eq!(
            fs::metadata(receipt.path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(fixture.coordination.lock_path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn patch_and_validation_failures_do_not_create_a_file() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = config_domain();
    let path = fixture.path_for(&domain);
    store.register(&domain).unwrap();

    let patch_error = store
        .mutate(&domain, options(), |_value| {
            Err(DomainIssue::new("rejected", "patch rejected"))
        })
        .unwrap_err();
    assert!(matches!(patch_error, MutationError::Patch(_)));
    assert!(!path.exists());

    let validation_error = store
        .mutate(&domain, options(), |value| {
            value.name.clear();
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(validation_error, MutationError::Validation(_)));
    assert!(!path.exists());
}

#[test]
fn migrated_and_recovery_sources_are_preserved() {
    let migrated_fixture = Fixture::new();
    let mut migrated_store = migrated_fixture.store();
    let migrated_domain = config_domain();
    let migrated_bytes = document("example.preferences", 1, json!({"label": "legacy"}));
    let migrated_path = migrated_fixture.write(&migrated_domain, &migrated_bytes);
    migrated_store.register(&migrated_domain).unwrap();

    let migrated_error = migrated_store
        .mutate(&migrated_domain, options(), |_value| Ok(()))
        .unwrap_err();
    assert!(matches!(
        migrated_error,
        MutationError::Refused(MutationRefusal::MigrationBackupRequired { .. })
    ));
    assert_eq!(fs::read(migrated_path).unwrap(), migrated_bytes);

    for (bytes, expected) in [
        (b"{not-json".to_vec(), RecoveryKind::CorruptDocument),
        (
            document(
                "example.preferences",
                4,
                json!({"name": "future", "enabled": true}),
            ),
            RecoveryKind::FutureSchema,
        ),
    ] {
        let fixture = Fixture::new();
        let mut store = fixture.store();
        let domain = config_domain();
        let path = fixture.write(&domain, &bytes);
        store.register(&domain).unwrap();

        let error = store
            .mutate(&domain, options(), |_value| Ok(()))
            .unwrap_err();
        let MutationError::Refused(MutationRefusal::Recovery(recovery)) = error else {
            panic!("expected recovery refusal");
        };
        assert_eq!(recovery.kind, expected);
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn non_file_read_only_and_project_authorities_are_refused() {
    let fixture = Fixture::new();
    let cases = [
        PreferencesDomain::new("example.defaults", StorageClass::Defaults, None, 3),
        PreferencesDomain::new("example.secret", StorageClass::Secret, None, 3),
        PreferencesDomain::new(
            "example.policy",
            StorageClass::Policy,
            Some("example/policy.json"),
            3,
        ),
        PreferencesDomain::new(
            "example.project",
            StorageClass::ProjectShared,
            Some("example/project.json"),
            3,
        ),
    ];

    for domain in cases {
        let mut store = fixture.store();
        store.register(&domain).unwrap();
        let error = store
            .mutate(&domain, options(), |_value| Ok(()))
            .unwrap_err();
        assert!(matches!(error, MutationError::Refused(_)));
    }
}

#[test]
fn nested_mutation_is_finitely_busy_not_deadlocked() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = config_domain();
    store.register(&domain).unwrap();
    let no_wait = MutationOptions::new(Duration::ZERO, DurabilityRequirement::Atomic);

    store
        .mutate(&domain, options(), |value| {
            let nested = store
                .mutate(&domain, no_wait, |_nested| Ok(()))
                .unwrap_err();
            let MutationError::Coordination(failure) = nested else {
                panic!("expected coordination failure");
            };
            assert_eq!(failure.kind, CoordinationFailureKind::Busy);
            value.name = "outer".to_owned();
            Ok(())
        })
        .unwrap();
}

#[cfg(unix)]
#[test]
fn capability_mutation_rejects_a_parent_symlink_escape() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = config_domain();
    let target = fixture.path_for(&domain);
    let outside = fixture.temp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, target.parent().unwrap()).unwrap();
    store.register(&domain).unwrap();

    assert!(store.mutate(&domain, options(), |_value| Ok(())).is_err());
    assert!(!outside.join("preferences.json").exists());
}
