use longhorn_settings::{
    SettingsLimits, SettingsMutationTiming, SettingsPageDefinition, SettingsPageFeatures,
    SettingsRegistryBuilder, SettingsRegistryGeneration, SettingsSectionDefinition,
};

use super::super::support::{
    capability_id, module_id, page_ids, register_capability, register_module_page, renderer_id,
    scope_id, section_id, unit_id,
};
use super::{page_with, two_module_registry};

#[test]
fn capability_admission_removes_page_and_empty_section() {
    let mut builder = super::super::support::minimal_builder();
    builder
        .register_section(SettingsSectionDefinition {
            id: section_id("app:optional-section"),
            module_id: module_id("app:module"),
            label: "Optional".into(),
            order: 20,
        })
        .unwrap();
    register_capability(&mut builder, "app", "app:hardware");
    builder
        .register_page(SettingsPageDefinition {
            section_id: section_id("app:optional-section"),
            required_capabilities: vec![capability_id("app:hardware")],
            ..page_with(
                module_id("app:module"),
                section_id("app:optional-section"),
                renderer_id("app:renderer"),
                vec![scope_id("app:scope")],
                vec![unit_id("app:apply")],
            )
        })
        .unwrap();

    let pruned = builder.clone().seal([]).unwrap();
    assert_eq!(page_ids(&pruned), vec!["app:page"]);
    assert_eq!(pruned.sections().count(), 1);

    let admitted = builder.seal([capability_id("app:hardware")]).unwrap();
    assert_eq!(admitted.pages().count(), 2);
    assert_eq!(admitted.sections().count(), 2);
    assert_ne!(admitted.digest(), pruned.digest());
}

#[test]
fn explicit_order_uses_stable_id_tie_break() {
    let mut builder = SettingsRegistryBuilder::new(
        SettingsRegistryGeneration::INITIAL,
        SettingsLimits::default(),
    );
    register_module_page(
        &mut builder,
        "zeta",
        "Zeta",
        10,
        SettingsMutationTiming::Immediate,
        &[],
        SettingsPageFeatures::default(),
    );
    register_module_page(
        &mut builder,
        "alpha",
        "Alpha",
        10,
        SettingsMutationTiming::Staged,
        &[],
        SettingsPageFeatures::default(),
    );

    let registry = builder.seal([]).unwrap();
    let modules: Vec<_> = registry
        .modules()
        .map(|module| module.id.as_str())
        .collect();
    assert_eq!(modules, vec!["alpha:module", "zeta:module"]);
}

#[test]
fn digest_ignores_registration_order_and_generation() {
    let forward = two_module_registry(false, 3);
    let reverse = two_module_registry(true, 99);

    assert_eq!(forward.digest(), reverse.digest());
    assert_ne!(forward.generation(), reverse.generation());
    assert_eq!(page_ids(&forward), page_ids(&reverse));
}
