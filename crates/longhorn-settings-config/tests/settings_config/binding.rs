use std::time::Duration;

use longhorn_config::StorageClass;
use longhorn_core::{SettingsApplyUnitId, SettingsPageId};
use longhorn_settings::{
    SettingsLoadOutcome, SettingsMutationResult, SettingsMutationTiming, SettingsPageFeatures,
    SettingsRegistryGeneration, SettingsRejectionCode,
};
use longhorn_settings_config::{ConfigSettingsApplyUnit, ConfigSettingsBindingError};
use serde_json::json;

use super::support::{
    Fixture, PolicyMode, PreferencesAdapter, PreferencesDomain, apply_command, entry_id,
    load_command, loaded_snapshot, opaque, options, registry_with_page, reset_command, unit_id,
};

#[test]
fn binding_reports_unknown_unit_missing_page_and_unsupported_storage() {
    let registry = registry_with_page(
        SettingsMutationTiming::Staged,
        Some(SettingsPageFeatures::default()),
    );
    let unreachable = registry_with_page(SettingsMutationTiming::Staged, None);
    let unknown = SettingsApplyUnitId::new("preferences:unknown").unwrap();

    for (registry, id, storage, expected) in [
        (
            &registry,
            &unknown,
            StorageClass::UserConfig,
            ConfigSettingsBindingError::UnknownApplyUnit {
                apply_unit_id: unknown.clone(),
            },
        ),
        (
            &unreachable,
            &unit_id(),
            StorageClass::UserConfig,
            ConfigSettingsBindingError::NoAuthorizedPage {
                apply_unit_id: unit_id(),
            },
        ),
        (
            &registry,
            &unit_id(),
            StorageClass::ProjectShared,
            ConfigSettingsBindingError::UnsupportedStorageClass {
                domain_id: longhorn_core::DomainId::new("preferences.settings").unwrap(),
                storage_class: StorageClass::ProjectShared,
            },
        ),
    ] {
        let domain =
            PreferencesDomain::with_storage("preferences.settings", "settings.json", storage);
        let (adapter, _) = PreferencesAdapter::new(PolicyMode::Editable);
        let result = ConfigSettingsApplyUnit::new(registry, id, domain, adapter);
        assert_eq!(result.err(), Some(expected));
    }
}

#[test]
fn generation_and_page_authorization_precede_reset_support() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let domain = PreferencesDomain::new("preferences.settings", "preferences/settings.json");
    store.register(&domain).unwrap();
    let registry = registry_with_page(
        SettingsMutationTiming::Staged,
        Some(SettingsPageFeatures::default()),
    );
    let (adapter, _) = PreferencesAdapter::new(PolicyMode::Editable);
    let unit = ConfigSettingsApplyUnit::new(&registry, &unit_id(), domain, adapter).unwrap();

    let mut read_only_load = load_command();
    read_only_load.page_id = SettingsPageId::new("preferences:reader").unwrap();
    let initial = loaded_snapshot(
        unit.load(&store, &read_only_load, Duration::from_secs(2))
            .unwrap(),
    );
    let unauthorized = SettingsPageId::new("preferences:display-only").unwrap();
    let stale = SettingsRegistryGeneration::INITIAL;
    let mut load = load_command();
    load.page_id = unauthorized.clone();
    load.registry_generation = stale;
    for expected in [
        SettingsRejectionCode::RegistryChanged,
        SettingsRejectionCode::Unauthorized,
    ] {
        let SettingsLoadOutcome::Rejected { rejection } =
            unit.load(&store, &load, Duration::ZERO).unwrap()
        else {
            panic!("expected load rejection {expected:?}");
        };
        assert_eq!(rejection.code, expected);
        load.registry_generation = registry.generation();
    }

    assert_rejected(
        unit.apply(
            &store,
            &{
                let mut command = apply_command(initial.authority.clone(), opaque(json!(null)));
                command.page_id = read_only_load.page_id;
                command
            },
            options(),
        )
        .unwrap(),
        SettingsRejectionCode::Unauthorized,
    );

    let mut apply = apply_command(initial.authority.clone(), opaque(json!(null)));
    apply.page_id = unauthorized.clone();
    apply.authority.registry_generation = stale;
    let mut reset = reset_command(
        initial.authority.clone(),
        vec![entry_id("preferences:theme")],
    );
    reset.page_id = unauthorized;
    reset.authority.registry_generation = stale;
    for expected in [
        SettingsRejectionCode::RegistryChanged,
        SettingsRejectionCode::Unauthorized,
    ] {
        assert_rejected(unit.apply(&store, &apply, options()).unwrap(), expected);
        assert_rejected(unit.reset(&store, &reset, options()).unwrap(), expected);
        apply.authority.registry_generation = registry.generation();
        reset.authority.registry_generation = registry.generation();
    }

    reset.page_id = load_command().page_id;
    assert_rejected(
        unit.reset(&store, &reset, options()).unwrap(),
        SettingsRejectionCode::Unauthorized,
    );
    assert!(!fixture.config_path().exists());
}

fn assert_rejected(result: SettingsMutationResult, expected: SettingsRejectionCode) {
    let SettingsMutationResult::Rejected {
        rejection,
        snapshot: None,
    } = result
    else {
        panic!("expected rejection without a snapshot, found {result:?}");
    };
    assert_eq!(rejection.code, expected);
}
