use longhorn_settings::{
    SettingsAnchorDefinition, SettingsApplyUnitDefinition, SettingsCapabilityDefinition,
    SettingsLimits, SettingsModuleDefinition, SettingsMutationTiming, SettingsPageDefinition,
    SettingsPageFeatures, SettingsRegistryBuilder, SettingsRegistryGeneration,
    SettingsRegistrySnapshot, SettingsRendererDefinition, SettingsScopeDefinition,
    SettingsSectionDefinition,
};

use super::ids::{
    anchor_id, capability_id, module_id, page_id, renderer_id, scope_id, section_id, unit_id,
};

pub(super) fn registry(limits: SettingsLimits) -> SettingsRegistrySnapshot {
    let mut builder = SettingsRegistryBuilder::new(SettingsRegistryGeneration::new(7), limits);
    builder
        .register_module(SettingsModuleDefinition {
            id: module_id("app:module"),
            label: "Application".into(),
            order: 0,
        })
        .unwrap();
    builder
        .register_section(SettingsSectionDefinition {
            id: section_id("app:general"),
            module_id: module_id("app:module"),
            label: "General".into(),
            order: 0,
        })
        .unwrap();
    builder
        .register_renderer(SettingsRendererDefinition {
            id: renderer_id("app:form"),
            module_id: module_id("app:module"),
        })
        .unwrap();
    builder
        .register_scope(SettingsScopeDefinition {
            id: scope_id("app:preferences"),
            module_id: module_id("app:module"),
        })
        .unwrap();
    builder
        .register_apply_unit(SettingsApplyUnitDefinition {
            id: unit_id("app:audio"),
            module_id: module_id("app:module"),
            scope_id: scope_id("app:preferences"),
            timing: SettingsMutationTiming::Staged,
            reset_supported: true,
        })
        .unwrap();
    builder
        .register_capability(SettingsCapabilityDefinition {
            id: capability_id("app:audio-capable"),
            module_id: module_id("app:module"),
        })
        .unwrap();
    builder
        .register_page(SettingsPageDefinition {
            id: page_id("app:audio"),
            module_id: module_id("app:module"),
            section_id: section_id("app:general"),
            renderer_id: renderer_id("app:form"),
            label: "Audio".into(),
            keywords: vec!["device".into(), "output".into()],
            order: 10,
            anchors: vec![SettingsAnchorDefinition {
                id: anchor_id("app:output"),
                label: Some("Output device".into()),
                order: 0,
            }],
            required_capabilities: vec![capability_id("app:audio-capable")],
            readable_scope_ids: vec![scope_id("app:preferences")],
            writable_apply_unit_ids: vec![unit_id("app:audio")],
            features: SettingsPageFeatures {
                reset: true,
                import: true,
                backup: true,
                restore: true,
                confirmation: true,
            },
        })
        .unwrap();
    SettingsRegistrySnapshot::from(&builder.seal([capability_id("app:audio-capable")]).unwrap())
}
