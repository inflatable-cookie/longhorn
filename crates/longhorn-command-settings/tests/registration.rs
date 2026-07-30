//! Capability-admission tests for the optional command settings module.

use longhorn_command_settings::{
    COMMAND_CATALOGUE_CAPABILITY_ID, KEYBINDING_SETTINGS_PAGE_ID, WRITABLE_KEYMAP_CAPABILITY_ID,
    register_command_settings,
};
use longhorn_core::SettingsCapabilityId;
use longhorn_settings::{SettingsLimits, SettingsRegistryBuilder, SettingsRegistryGeneration};

#[test]
fn keybinding_navigation_requires_both_composed_capabilities() {
    for composed in [
        vec![],
        vec![capability(COMMAND_CATALOGUE_CAPABILITY_ID)],
        vec![capability(WRITABLE_KEYMAP_CAPABILITY_ID)],
    ] {
        let registry = registry(composed);
        assert!(
            registry
                .pages()
                .all(|page| page.id.as_str() != KEYBINDING_SETTINGS_PAGE_ID)
        );
        assert_eq!(registry.sections().count(), 0);
    }

    let registry = registry([
        capability(COMMAND_CATALOGUE_CAPABILITY_ID),
        capability(WRITABLE_KEYMAP_CAPABILITY_ID),
    ]);
    let page = registry
        .pages()
        .find(|page| page.id.as_str() == KEYBINDING_SETTINGS_PAGE_ID)
        .expect("keybinding page");
    assert!(!page.features.reset);
    assert!(page.readable_scope_ids.is_empty());
    assert!(page.writable_apply_unit_ids.is_empty());
}

#[test]
fn command_settings_registration_is_explicit_and_duplicate_safe() {
    let mut builder = builder();
    register_command_settings(&mut builder).expect("first registration");
    assert!(register_command_settings(&mut builder).is_err());
}

fn registry(
    composed: impl IntoIterator<Item = SettingsCapabilityId>,
) -> longhorn_settings::SettingsRegistry {
    let mut builder = builder();
    register_command_settings(&mut builder).expect("registration");
    builder.seal(composed).expect("sealed registry")
}

fn builder() -> SettingsRegistryBuilder {
    SettingsRegistryBuilder::new(
        SettingsRegistryGeneration::INITIAL,
        SettingsLimits::default(),
    )
}

fn capability(value: &str) -> SettingsCapabilityId {
    SettingsCapabilityId::new(value).expect("capability id")
}
