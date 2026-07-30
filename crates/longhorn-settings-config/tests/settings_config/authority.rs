use std::{fs, process::Command, sync::atomic::Ordering, time::Duration};

use longhorn_config::{ConfigStore, LoadOutcome, MutationOptions};
use longhorn_settings::{
    SettingsDurabilityEvidence, SettingsMutationOutcome, SettingsMutationResult,
    SettingsMutationTiming,
};
use longhorn_settings_config::ConfigSettingsApplyUnit;
use serde_json::json;

use super::support::{
    Fixture, PolicyMode, PreferencesAdapter, PreferencesDomain, applied, apply_command,
    load_command, loaded_snapshot, opaque, options, sealed_registry, unit_id,
};

fn authority(
    timing: SettingsMutationTiming,
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
    let registry = sealed_registry(timing);
    let (adapter, activation_calls) = PreferencesAdapter::new(PolicyMode::Editable);
    let unit = ConfigSettingsApplyUnit::new(&registry, &unit_id(), domain, adapter).unwrap();
    (fixture, store, unit, activation_calls)
}

#[test]
fn immediate_and_staged_share_fresh_apply_and_exact_receipts() {
    for timing in [
        SettingsMutationTiming::Immediate,
        SettingsMutationTiming::Staged,
    ] {
        let (_fixture, store, unit, activation_calls) = authority(timing);
        let initial = loaded_snapshot(
            unit.load(&store, &load_command(), Duration::from_secs(2))
                .unwrap(),
        );
        assert_eq!(initial.authority.scope_revision.get(), 0);
        assert!(initial.values.iter().all(|value| {
            value
                .source_diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "config:missing")
        }));

        let command = apply_command(
            initial.authority.clone(),
            opaque(json!({
                "entryId": "preferences:theme",
                "value": "dark"
            })),
        );
        let (snapshot, receipt) = applied(unit.apply(&store, &command, options()).unwrap());
        assert_eq!(receipt.outcome, SettingsMutationOutcome::Changed);
        assert_eq!(snapshot.authority.scope_revision.get(), 1);
        assert_ne!(
            snapshot.authority.authority_token,
            initial.authority.authority_token
        );
        assert_eq!(receipt.activation_requirements.len(), 1);
        assert_eq!(
            snapshot.activation_requirements,
            receipt.activation_requirements
        );
        let SettingsDurabilityEvidence::Confirmed {
            evidence: Some(evidence),
        } = receipt.durability
        else {
            panic!("expected exact config durability evidence");
        };
        assert_eq!(
            evidence.value(),
            &json!({"configDurability": "fileAndDirectorySynced"})
        );
        assert_eq!(activation_calls.load(Ordering::SeqCst), 1);

        let loaded = store.load(unit.domain()).unwrap();
        let LoadOutcome::Ready(loaded) = loaded else {
            panic!("expected committed preferences");
        };
        assert_eq!(loaded.value.theme.as_deref(), Some("dark"));
    }
}

#[test]
fn unchanged_apply_has_no_publication_and_still_returns_fresh_authority() {
    let (fixture, store, unit, activation_calls) = authority(SettingsMutationTiming::Immediate);
    let initial = loaded_snapshot(
        unit.load(&store, &load_command(), Duration::from_secs(2))
            .unwrap(),
    );
    let (current, _) = applied(
        unit.apply(
            &store,
            &apply_command(
                initial.authority,
                opaque(json!({
                    "entryId": "preferences:theme",
                    "value": "same"
                })),
            ),
            options(),
        )
        .unwrap(),
    );
    let command = apply_command(
        current.authority,
        opaque(json!({
            "entryId": "preferences:theme",
            "value": "same"
        })),
    );
    let before = fs::read(fixture.config_path()).unwrap();
    let (_snapshot, receipt) = applied(unit.apply(&store, &command, options()).unwrap());
    assert_eq!(receipt.outcome, SettingsMutationOutcome::Unchanged);
    assert_eq!(
        receipt.durability,
        SettingsDurabilityEvidence::NotApplicable
    );
    assert_eq!(fs::read(fixture.config_path()).unwrap(), before);
    assert_eq!(activation_calls.load(Ordering::SeqCst), 2);
}

#[test]
fn intervening_process_writer_returns_conflict_without_publication() {
    let (fixture, store, unit, activation_calls) = authority(SettingsMutationTiming::Staged);
    let initial = loaded_snapshot(
        unit.load(&store, &load_command(), Duration::from_secs(2))
            .unwrap(),
    );
    let status = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("authority::intervening_writer_helper")
        .arg("--nocapture")
        .env("LONGHORN_SETTINGS_WRITER_ROOT", fixture.temp.path())
        .status()
        .unwrap();
    assert!(status.success());
    let intervening_bytes = fs::read(fixture.config_path()).unwrap();

    let command = apply_command(
        initial.authority.clone(),
        opaque(json!({
            "entryId": "preferences:theme",
            "value": "stale-renderer"
        })),
    );
    let result = unit.apply(&store, &command, options()).unwrap();
    let SettingsMutationResult::Conflict { conflict, snapshot } = result else {
        panic!("expected stale authority conflict");
    };
    assert_eq!(conflict.expected, initial.authority);
    assert_ne!(conflict.actual, conflict.expected);
    assert_eq!(snapshot.authority, conflict.actual);
    assert_eq!(fs::read(fixture.config_path()).unwrap(), intervening_bytes);
    assert_eq!(activation_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn intervening_writer_helper() {
    let Some(root) = std::env::var_os("LONGHORN_SETTINGS_WRITER_ROOT") else {
        return;
    };
    let domain = PreferencesDomain::new("preferences.settings", "preferences/settings.json");
    let mut store = Fixture::store_at(std::path::Path::new(&root));
    store.register(&domain).unwrap();
    store
        .mutate(
            &domain,
            MutationOptions::new(
                Duration::from_secs(2),
                longhorn_config::DurabilityRequirement::Durable,
            ),
            |preferences| {
                preferences.theme = Some("external-writer".into());
                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn checked_config_seam_exposes_exact_fresh_source_under_the_patch_call() {
    let (fixture, store, unit, _activation_calls) = authority(SettingsMutationTiming::Immediate);
    let initial = loaded_snapshot(
        unit.load(&store, &load_command(), Duration::from_secs(2))
            .unwrap(),
    );
    unit.apply(
        &store,
        &apply_command(
            initial.authority,
            opaque(json!({"entryId": "preferences:theme", "value": "source"})),
        ),
        options(),
    )
    .unwrap();
    let expected = fs::read(fixture.config_path()).unwrap();
    let outcome = store
        .mutate_checked(unit.domain(), options(), |context| {
            assert_eq!(context.source().unwrap().bytes, expected);
            Ok::<_, ()>(())
        })
        .unwrap();
    assert!(!outcome.changed());
}
