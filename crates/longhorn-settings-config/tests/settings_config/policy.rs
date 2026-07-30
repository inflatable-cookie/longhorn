use std::{fs, sync::atomic::Ordering, time::Duration};

use longhorn_config::{ConfigStore, LoadOutcome};
use longhorn_settings::{
    SettingsEffectiveSource, SettingsMutationResult, SettingsMutationTiming, SettingsRejectionCode,
};
use longhorn_settings_config::ConfigSettingsApplyUnit;
use serde_json::json;

use super::support::{
    Fixture, PolicyMode, PreferencesAdapter, PreferencesDomain, applied, apply_command, entry_id,
    load_command, loaded_snapshot, opaque, options, reset_command, sealed_registry, unit_id,
};

fn authority(
    policy: PolicyMode,
) -> (
    Fixture,
    ConfigStore,
    ConfigSettingsApplyUnit<PreferencesDomain, PreferencesAdapter>,
    std::sync::Arc<std::sync::atomic::AtomicUsize>,
) {
    let fixture = Fixture::new();
    let domain = PreferencesDomain::new("preferences.settings", "preferences/settings.json");
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let registry = sealed_registry(SettingsMutationTiming::Staged);
    let (adapter, calls) = PreferencesAdapter::new(policy);
    let unit = ConfigSettingsApplyUnit::new(&registry, &unit_id(), domain, adapter).unwrap();
    (fixture, store, unit, calls)
}

#[test]
fn forced_policy_keeps_configured_value_visible_but_blocks_mutation() {
    let (fixture, store, unit, activation_calls) = authority(PolicyMode::ForcedTheme);
    store
        .mutate(unit.domain(), options(), |value| {
            value.theme = Some("user-light".into());
            Ok(())
        })
        .unwrap();
    let snapshot = loaded_snapshot(
        unit.load(&store, &load_command(), Duration::from_secs(2))
            .unwrap(),
    );
    let theme = snapshot
        .values
        .iter()
        .find(|value| value.entry_id == entry_id("preferences:theme"))
        .unwrap();
    assert_eq!(
        theme.configured.as_ref().unwrap().value(),
        &json!("user-light")
    );
    assert_eq!(theme.effective.value(), &json!("managed-dark"));
    assert_eq!(
        theme.effective_source,
        SettingsEffectiveSource::ManagedPolicy
    );
    let before = fs::read(fixture.config_path()).unwrap();

    let result = unit
        .apply(
            &store,
            &apply_command(
                snapshot.authority,
                opaque(json!({
                    "entryId": "preferences:theme",
                    "value": "renderer-write"
                })),
            ),
            options(),
        )
        .unwrap();
    assert_rejection(result, SettingsRejectionCode::PolicyBlocked);
    assert_eq!(fs::read(fixture.config_path()).unwrap(), before);
    assert_eq!(activation_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn constrained_read_only_hidden_and_unsupported_targets_fail_before_patch() {
    let cases = [
        (
            PolicyMode::Editable,
            "preferences:volume",
            json!(11),
            SettingsRejectionCode::PolicyBlocked,
        ),
        (
            PolicyMode::ReadOnlyTheme,
            "preferences:theme",
            json!("dark"),
            SettingsRejectionCode::ReadOnly,
        ),
        (
            PolicyMode::Editable,
            "preferences:hidden",
            json!("changed"),
            SettingsRejectionCode::Hidden,
        ),
        (
            PolicyMode::Editable,
            "preferences:unsupported",
            json!("changed"),
            SettingsRejectionCode::Unsupported,
        ),
    ];
    for (policy, entry, value, expected) in cases {
        let (fixture, store, unit, activation_calls) = authority(policy);
        let snapshot = loaded_snapshot(
            unit.load(&store, &load_command(), Duration::from_secs(2))
                .unwrap(),
        );
        let result = unit
            .apply(
                &store,
                &apply_command(
                    snapshot.authority,
                    opaque(json!({"entryId": entry, "value": value})),
                ),
                options(),
            )
            .unwrap();
        assert_rejection(result, expected);
        assert!(!fixture.config_path().exists());
        assert_eq!(activation_calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn invalid_staged_intent_changes_no_bytes() {
    let (fixture, store, unit, activation_calls) = authority(PolicyMode::Editable);
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
    let before = fs::read(fixture.config_path()).unwrap();
    let result = unit
        .apply(
            &store,
            &apply_command(
                snapshot.authority,
                opaque(json!({
                    "entryId": "preferences:theme",
                    "value": 42
                })),
            ),
            options(),
        )
        .unwrap();
    assert_rejection(result, SettingsRejectionCode::InvalidIntent);
    assert_eq!(fs::read(fixture.config_path()).unwrap(), before);
    assert_eq!(activation_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn reset_removes_only_named_override_and_preserves_other_authorities() {
    let (fixture, mut store, unit, activation_calls) = authority(PolicyMode::Editable);
    let other_domain = PreferencesDomain::new("preferences.other", "preferences/other.json");
    store.register(&other_domain).unwrap();
    store
        .mutate(unit.domain(), options(), |value| {
            value.theme = Some("dark".into());
            value.volume = Some(8);
            Ok(())
        })
        .unwrap();
    store
        .mutate(&other_domain, options(), |value| {
            value.theme = Some("other-domain".into());
            Ok(())
        })
        .unwrap();
    let other_path = fixture.temp.path().join("config/preferences/other.json");
    let other_before = fs::read(&other_path).unwrap();
    let snapshot = loaded_snapshot(
        unit.load(&store, &load_command(), Duration::from_secs(2))
            .unwrap(),
    );
    let (committed, receipt) = applied(
        unit.reset(
            &store,
            &reset_command(snapshot.authority, vec![entry_id("preferences:theme")]),
            options(),
        )
        .unwrap(),
    );
    assert_eq!(receipt.activation_requirements.len(), 1);
    let LoadOutcome::Ready(loaded) = store.load(unit.domain()).unwrap() else {
        panic!("expected reset preferences");
    };
    assert_eq!(loaded.value.theme, None);
    assert_eq!(loaded.value.volume, Some(8));
    assert_eq!(loaded.value.secret, "secret-authority");
    assert_eq!(loaded.value.untouched, "other-domain-shape");
    assert_eq!(fs::read(other_path).unwrap(), other_before);
    let theme = committed
        .values
        .iter()
        .find(|value| value.entry_id == entry_id("preferences:theme"))
        .unwrap();
    assert_eq!(theme.compiled_default.value(), &json!("system"));
    assert!(theme.policy.is_none());
    assert_eq!(activation_calls.load(Ordering::SeqCst), 1);
}

fn assert_rejection(result: SettingsMutationResult, code: SettingsRejectionCode) {
    let SettingsMutationResult::Rejected { rejection, .. } = result else {
        panic!("expected settings rejection");
    };
    assert_eq!(rejection.code, code);
}
