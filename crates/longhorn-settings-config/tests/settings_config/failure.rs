use std::{fs, sync::atomic::Ordering, time::Duration};

use longhorn_config::MutationError;
use longhorn_settings::{
    SettingsMutationResult, SettingsMutationTiming, SettingsRecoveryCode, SettingsRejectionCode,
};
use longhorn_settings_config::{ConfigSettingsApplyUnit, SettingsConfigError};
use serde_json::json;

use super::support::{
    Fixture, PolicyMode, PreferencesAdapter, PreferencesDomain, apply_command, load_command,
    loaded_snapshot, opaque, options, sealed_registry, unit_id,
};

fn authority() -> (
    Fixture,
    longhorn_config::ConfigStore,
    ConfigSettingsApplyUnit<PreferencesDomain, PreferencesAdapter>,
    std::sync::Arc<std::sync::atomic::AtomicUsize>,
) {
    let fixture = Fixture::new();
    let domain = PreferencesDomain::new("preferences.settings", "preferences/settings.json");
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let registry = sealed_registry(SettingsMutationTiming::Staged);
    let (adapter, calls) = PreferencesAdapter::new(PolicyMode::Editable);
    let unit = ConfigSettingsApplyUnit::new(&registry, &unit_id(), domain, adapter).unwrap();
    (fixture, store, unit, calls)
}

#[test]
fn corrupt_source_projects_recovery_and_rejects_without_rewrite() {
    let (fixture, store, unit, activation_calls) = authority();
    fs::create_dir_all(fixture.config_path().parent().unwrap()).unwrap();
    fs::write(fixture.config_path(), b"{broken").unwrap();
    let before = fs::read(fixture.config_path()).unwrap();
    let snapshot = loaded_snapshot(
        unit.load(&store, &load_command(), Duration::from_secs(2))
            .unwrap(),
    );
    assert_eq!(
        snapshot.recovery.as_ref().map(|state| state.code),
        Some(SettingsRecoveryCode::Corrupt)
    );

    let result = unit
        .apply(
            &store,
            &apply_command(
                snapshot.authority,
                opaque(json!({
                    "entryId": "preferences:theme",
                    "value": "cannot-write-recovery"
                })),
            ),
            options(),
        )
        .unwrap();
    let SettingsMutationResult::Rejected {
        rejection,
        snapshot: Some(recovery),
    } = result
    else {
        panic!("expected recovery rejection with snapshot");
    };
    assert_eq!(rejection.code, SettingsRejectionCode::RecoveryRequired);
    assert_eq!(
        recovery.recovery.as_ref().map(|state| state.code),
        Some(SettingsRecoveryCode::Corrupt)
    );
    assert_eq!(fs::read(fixture.config_path()).unwrap(), before);
    assert_eq!(activation_calls.load(Ordering::SeqCst), 0);
}

#[cfg(unix)]
#[test]
fn publication_failure_preserves_bytes_and_never_runs_activation() {
    use std::os::unix::fs::PermissionsExt;

    let (fixture, store, unit, activation_calls) = authority();
    store
        .mutate(unit.domain(), options(), |value| {
            value.theme = Some("stable".into());
            Ok(())
        })
        .unwrap();
    let snapshot = loaded_snapshot(
        unit.load(&store, &load_command(), Duration::from_secs(2))
            .unwrap(),
    );
    let path = fixture.config_path();
    let parent = path.parent().unwrap();
    let before = fs::read(&path).unwrap();
    let original_mode = fs::metadata(parent).unwrap().permissions().mode();
    fs::set_permissions(parent, fs::Permissions::from_mode(0o500)).unwrap();
    let result = unit.apply(
        &store,
        &apply_command(
            snapshot.authority,
            opaque(json!({
                "entryId": "preferences:theme",
                "value": "publication-must-fail"
            })),
        ),
        options(),
    );
    fs::set_permissions(parent, fs::Permissions::from_mode(original_mode)).unwrap();

    let Err(SettingsConfigError::Mutation(MutationError::Publication(failure))) = result else {
        panic!("expected atomic publication failure, found {result:?}");
    };
    assert!(!failure.published);
    assert_eq!(fs::read(path).unwrap(), before);
    assert_eq!(activation_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn recovery_token_changes_with_exact_source_bytes() {
    let (fixture, store, unit, _activation_calls) = authority();
    fs::create_dir_all(fixture.config_path().parent().unwrap()).unwrap();
    fs::write(fixture.config_path(), b"{first").unwrap();
    let first = loaded_snapshot(
        unit.load(&store, &load_command(), Duration::from_secs(2))
            .unwrap(),
    );
    fs::write(fixture.config_path(), b"{second").unwrap();
    let second = loaded_snapshot(
        unit.load(&store, &load_command(), Duration::from_secs(2))
            .unwrap(),
    );
    assert_ne!(
        first.authority.authority_token,
        second.authority.authority_token
    );
    assert!(second.authority.scope_revision > first.authority.scope_revision);
}

#[test]
fn invalid_config_value_returns_typed_rejection_without_activation_or_publication() {
    let (fixture, store, unit, activation_calls) = authority();
    let initial = loaded_snapshot(
        unit.load(&store, &load_command(), Duration::from_secs(2))
            .unwrap(),
    );
    let command = apply_command(
        initial.authority.clone(),
        opaque(json!({"entryId": "preferences:theme", "value": "domain-refused"})),
    );

    assert_rejected(
        unit.apply(&store, &command, options()).unwrap(),
        SettingsRejectionCode::InvalidIntent,
    );
    let after = loaded_snapshot(
        unit.load(&store, &load_command(), Duration::from_secs(2))
            .unwrap(),
    );
    assert_eq!(after, initial);
    assert!(!fixture.config_path().exists());
    assert_eq!(activation_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn unavailable_workspace_authority_returns_typed_recovery() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = PreferencesDomain::with_storage(
        "preferences.settings",
        "preferences/settings.json",
        longhorn_config::StorageClass::WorkspaceLocal,
    );
    store.register(&domain).unwrap();
    let registry = sealed_registry(SettingsMutationTiming::Staged);
    let (adapter, calls) = PreferencesAdapter::new(PolicyMode::Editable);
    let unit = ConfigSettingsApplyUnit::new(&registry, &unit_id(), domain, adapter).unwrap();
    let initial = loaded_snapshot(
        unit.load(&store, &load_command(), Duration::from_secs(2))
            .unwrap(),
    );
    let command = apply_command(initial.authority.clone(), opaque(json!(null)));

    let SettingsMutationResult::Rejected {
        rejection,
        snapshot: Some(snapshot),
    } = unit.apply(&store, &command, options()).unwrap()
    else {
        panic!("expected recovery snapshot");
    };
    assert_eq!(rejection.code, SettingsRejectionCode::RecoveryRequired);
    assert_eq!(
        snapshot.recovery.unwrap().code,
        SettingsRecoveryCode::AuthorityUnavailable
    );
    assert_eq!(snapshot.authority, initial.authority);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn migrated_config_returns_typed_recovery_and_preserves_source() {
    let fixture = Fixture::new();
    let path = fixture.config_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let source = serde_json::to_vec(&json!({
        "domain": "preferences.settings",
        "schemaVersion": 1,
        "value": super::support::Preferences::default(),
    }))
    .unwrap();
    fs::write(&path, &source).unwrap();
    let mut store = fixture.store();
    let domain = PreferencesDomain::with_version(
        "preferences.settings",
        "preferences/settings.json",
        longhorn_config::StorageClass::UserConfig,
        2,
    );
    store.register(&domain).unwrap();
    let registry = sealed_registry(SettingsMutationTiming::Staged);
    let (adapter, calls) = PreferencesAdapter::new(PolicyMode::Editable);
    let unit = ConfigSettingsApplyUnit::new(&registry, &unit_id(), domain, adapter).unwrap();
    let initial = loaded_snapshot(
        unit.load(&store, &load_command(), Duration::from_secs(2))
            .unwrap(),
    );
    let command = apply_command(initial.authority, opaque(json!(null)));

    assert_rejected(
        unit.apply(&store, &command, options()).unwrap(),
        SettingsRejectionCode::RecoveryRequired,
    );
    assert_eq!(fs::read(path).unwrap(), source);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

fn assert_rejected(result: SettingsMutationResult, code: SettingsRejectionCode) {
    let SettingsMutationResult::Rejected {
        rejection,
        snapshot: None,
    } = result
    else {
        panic!("expected a typed rejection, found {result:?}");
    };
    assert_eq!(rejection.code, code);
}
