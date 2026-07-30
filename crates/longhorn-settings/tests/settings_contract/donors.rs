use longhorn_settings::{
    SettingsLimits, SettingsMutationTiming, SettingsPageFeatures, SettingsRegistryBuilder,
    SettingsRegistryGeneration,
};

use super::support::{capability_id, page_ids, register_capability, register_module_page};

#[test]
fn bovine_fixture_is_one_staged_page_without_optional_systems() {
    let mut builder = SettingsRegistryBuilder::new(
        SettingsRegistryGeneration::new(1),
        SettingsLimits::default(),
    );
    register_module_page(
        &mut builder,
        "bovine",
        "Workspace",
        0,
        SettingsMutationTiming::Staged,
        &[],
        SettingsPageFeatures {
            reset: true,
            ..SettingsPageFeatures::default()
        },
    );

    let registry = builder.seal([]).unwrap();
    assert_eq!(page_ids(&registry), vec!["bovine:page"]);
    assert_eq!(
        registry.apply_units().next().unwrap().timing,
        SettingsMutationTiming::Staged
    );
}

#[test]
fn soundcheck_fixture_composes_product_and_optional_recovery_pages() {
    let mut builder = SettingsRegistryBuilder::new(
        SettingsRegistryGeneration::new(4),
        SettingsLimits::default(),
    );
    register_module_page(
        &mut builder,
        "soundcheck",
        "Audio",
        0,
        SettingsMutationTiming::Immediate,
        &[],
        SettingsPageFeatures {
            reset: true,
            ..SettingsPageFeatures::default()
        },
    );
    register_module_page(
        &mut builder,
        "soundcheck-library",
        "Library",
        10,
        SettingsMutationTiming::Staged,
        &[],
        SettingsPageFeatures::default(),
    );
    register_module_page(
        &mut builder,
        "soundcheck-recovery",
        "Backup and Restore",
        20,
        SettingsMutationTiming::Staged,
        &["soundcheck-recovery:available"],
        SettingsPageFeatures {
            backup: true,
            restore: true,
            confirmation: true,
            ..SettingsPageFeatures::default()
        },
    );
    register_capability(
        &mut builder,
        "soundcheck-recovery",
        "soundcheck-recovery:available",
    );

    let without_recovery = builder.clone().seal([]).unwrap();
    assert_eq!(
        page_ids(&without_recovery),
        vec!["soundcheck:page", "soundcheck-library:page"]
    );
    let with_recovery = builder
        .seal([capability_id("soundcheck-recovery:available")])
        .unwrap();
    assert_eq!(with_recovery.pages().count(), 3);
}

#[test]
fn loophole_fixture_keeps_hardware_and_keybindings_consumer_owned() {
    let mut builder = SettingsRegistryBuilder::new(
        SettingsRegistryGeneration::new(9),
        SettingsLimits::default(),
    );
    register_module_page(
        &mut builder,
        "loophole",
        "Appearance",
        0,
        SettingsMutationTiming::Immediate,
        &[],
        SettingsPageFeatures::default(),
    );
    register_module_page(
        &mut builder,
        "loophole-hardware",
        "Hardware",
        10,
        SettingsMutationTiming::Immediate,
        &["loophole-hardware:available"],
        SettingsPageFeatures::default(),
    );
    register_module_page(
        &mut builder,
        "loophole-keybindings",
        "Keybindings",
        20,
        SettingsMutationTiming::Staged,
        &["loophole-keybindings:available"],
        SettingsPageFeatures {
            reset: true,
            ..SettingsPageFeatures::default()
        },
    );
    register_capability(
        &mut builder,
        "loophole-hardware",
        "loophole-hardware:available",
    );
    register_capability(
        &mut builder,
        "loophole-keybindings",
        "loophole-keybindings:available",
    );

    let registry = builder
        .seal([
            capability_id("loophole-hardware:available"),
            capability_id("loophole-keybindings:available"),
        ])
        .unwrap();

    assert_eq!(registry.pages().count(), 3);
    assert_eq!(registry.scopes().count(), 3);
    assert_eq!(registry.apply_units().count(), 3);
}
