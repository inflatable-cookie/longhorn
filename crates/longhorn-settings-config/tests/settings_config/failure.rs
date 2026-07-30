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
