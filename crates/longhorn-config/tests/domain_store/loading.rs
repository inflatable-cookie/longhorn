use std::fs;

use longhorn_config::{LoadDiagnosticCode, LoadOutcome, LoadedOrigin, RecoveryKind};
use longhorn_core::SchemaVersion;
use serde_json::json;

use super::common::{Fixture, MigrationBehavior, Preferences, config_domain, document};

#[test]
fn missing_file_returns_a_validated_default_without_writing() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = config_domain();
    let path = fixture.path_for(&domain);
    store.register(&domain).unwrap();

    let LoadOutcome::Ready(loaded) = store.load(&domain).unwrap() else {
        panic!("expected ready default");
    };

    assert_eq!(
        loaded.value,
        Preferences {
            name: "default".to_owned(),
            enabled: true
        }
    );
    assert_eq!(loaded.origin, LoadedOrigin::Default);
    assert_eq!(loaded.diagnostics.len(), 1);
    assert_eq!(loaded.diagnostics[0].code, LoadDiagnosticCode::Missing);
    assert!(loaded.source.is_none());
    assert!(!path.exists());
}

#[test]
fn invalid_default_enters_recovery() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = config_domain().with_invalid_default();
    store.register(&domain).unwrap();

    let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
        panic!("expected recovery");
    };

    assert_eq!(recovery.kind, RecoveryKind::InvalidDefault);
    assert!(recovery.source.is_none());
}

#[test]
fn current_document_decodes_and_preserves_source() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = config_domain();
    let bytes = document(
        "example.preferences",
        3,
        json!({"name": "current", "enabled": false}),
    );
    fixture.write(&domain, &bytes);
    store.register(&domain).unwrap();

    let LoadOutcome::Ready(loaded) = store.load(&domain).unwrap() else {
        panic!("expected ready file");
    };

    assert_eq!(
        loaded.value,
        Preferences {
            name: "current".to_owned(),
            enabled: false
        }
    );
    assert_eq!(loaded.origin, LoadedOrigin::File);
    assert_eq!(loaded.source.unwrap().bytes, bytes);
}

#[test]
fn old_document_migrates_in_memory_without_rewriting() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = config_domain();
    let bytes = document("example.preferences", 1, json!({"label": "legacy"}));
    let path = fixture.write(&domain, &bytes);
    store.register(&domain).unwrap();

    let LoadOutcome::Ready(loaded) = store.load(&domain).unwrap() else {
        panic!("expected migrated value");
    };

    assert_eq!(
        loaded.value,
        Preferences {
            name: "legacy".to_owned(),
            enabled: true
        }
    );
    assert_eq!(
        loaded.origin,
        LoadedOrigin::MigratedInMemory {
            from: SchemaVersion::new(1).unwrap(),
            to: SchemaVersion::new(3).unwrap()
        }
    );
    assert_eq!(loaded.source.unwrap().bytes, bytes);
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn invalid_sources_enter_typed_recovery_without_mutation() {
    let cases = [
        (
            "corrupt",
            b"{ definitely not json".to_vec(),
            RecoveryKind::CorruptDocument,
        ),
        (
            "mismatch",
            document(
                "example.somewhere-else",
                3,
                json!({"name": "valid", "enabled": true}),
            ),
            RecoveryKind::DomainMismatch,
        ),
        (
            "invalid",
            document("example.preferences", 3, json!({"name": "missing-enabled"})),
            RecoveryKind::InvalidValue,
        ),
        (
            "future",
            document(
                "example.preferences",
                4,
                json!({"name": "future", "enabled": true}),
            ),
            RecoveryKind::FutureSchema,
        ),
    ];

    for (name, bytes, expected_kind) in cases {
        let fixture = Fixture::new();
        let mut store = fixture.store();
        let domain = config_domain();
        let path = fixture.write(&domain, &bytes);
        store.register(&domain).unwrap();

        let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
            panic!("{name}: expected recovery");
        };

        assert_eq!(recovery.kind, expected_kind, "{name}");
        assert_eq!(recovery.source.unwrap().bytes, bytes, "{name}");
        assert_eq!(fs::read(path).unwrap(), bytes, "{name}");
    }
}

#[test]
fn incomplete_or_invalid_migration_preserves_source() {
    let cases = [
        (
            MigrationBehavior::MissingSecond,
            RecoveryKind::MissingMigration,
        ),
        (
            MigrationBehavior::WrongTarget,
            RecoveryKind::InvalidMigrationStep,
        ),
        (MigrationBehavior::FailSecond, RecoveryKind::MigrationFailed),
    ];

    for (behavior, expected_kind) in cases {
        let fixture = Fixture::new();
        let mut store = fixture.store();
        let domain = config_domain().with_behavior(behavior);
        let bytes = document("example.preferences", 1, json!({"label": "legacy"}));
        let path = fixture.write(&domain, &bytes);
        store.register(&domain).unwrap();

        let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
            panic!("expected recovery");
        };

        assert_eq!(recovery.kind, expected_kind);
        assert_eq!(recovery.source.unwrap().bytes, bytes);
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[cfg(unix)]
#[test]
fn capability_read_rejects_a_symlink_escape() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = config_domain();
    let domain_path = fixture.path_for(&domain);
    let outside_path = fixture.temp.path().join("outside.json");
    let outside_bytes = document(
        "example.preferences",
        3,
        json!({"name": "outside", "enabled": true}),
    );
    fs::write(&outside_path, &outside_bytes).unwrap();
    fs::create_dir_all(domain_path.parent().unwrap()).unwrap();
    symlink(&outside_path, &domain_path).unwrap();
    store.register(&domain).unwrap();

    let LoadOutcome::Recovery(recovery) = store.load(&domain).unwrap() else {
        panic!("expected confined read failure");
    };

    assert_eq!(recovery.kind, RecoveryKind::ReadFailed);
    assert!(recovery.source.is_none());
    assert_eq!(fs::read(outside_path).unwrap(), outside_bytes);
}
