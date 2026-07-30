use longhorn_core::{
    SettingsAnchorId, SettingsApplyUnitId, SettingsCapabilityId, SettingsModuleId, SettingsPageId,
    SettingsRendererId, SettingsScopeId, SettingsSectionId,
};
use longhorn_settings::{
    SettingsAnchorDefinition, SettingsApplyUnitDefinition, SettingsCapabilityDefinition,
    SettingsLimits, SettingsModuleDefinition, SettingsMutationTiming, SettingsPageDefinition,
    SettingsPageFeatures, SettingsRegistry, SettingsRegistryBuilder, SettingsRegistryGeneration,
    SettingsRendererDefinition, SettingsScopeDefinition, SettingsSectionDefinition,
};

pub fn module_id(value: &str) -> SettingsModuleId {
    SettingsModuleId::new(value).unwrap()
}

pub fn section_id(value: &str) -> SettingsSectionId {
    SettingsSectionId::new(value).unwrap()
}

pub fn page_id(value: &str) -> SettingsPageId {
    SettingsPageId::new(value).unwrap()
}

pub fn renderer_id(value: &str) -> SettingsRendererId {
    SettingsRendererId::new(value).unwrap()
}

pub fn scope_id(value: &str) -> SettingsScopeId {
    SettingsScopeId::new(value).unwrap()
}

pub fn unit_id(value: &str) -> SettingsApplyUnitId {
    SettingsApplyUnitId::new(value).unwrap()
}

pub fn capability_id(value: &str) -> SettingsCapabilityId {
    SettingsCapabilityId::new(value).unwrap()
}

pub fn anchor_id(value: &str) -> SettingsAnchorId {
    SettingsAnchorId::new(value).unwrap()
}

pub fn minimal_builder() -> SettingsRegistryBuilder {
    let mut builder = SettingsRegistryBuilder::new(
        SettingsRegistryGeneration::new(7),
        SettingsLimits::default(),
    );
    register_module_page(
        &mut builder,
        "app",
        "general",
        10,
        SettingsMutationTiming::Staged,
        &[],
        SettingsPageFeatures {
            reset: true,
            ..SettingsPageFeatures::default()
        },
    );
    builder
}

pub fn register_module_page(
    builder: &mut SettingsRegistryBuilder,
    prefix: &str,
    label: &str,
    order: i32,
    timing: SettingsMutationTiming,
    required_capabilities: &[&str],
    features: SettingsPageFeatures,
) {
    let module = module_id(&format!("{prefix}:module"));
    let section = section_id(&format!("{prefix}:section"));
    let renderer = renderer_id(&format!("{prefix}:renderer"));
    let scope = scope_id(&format!("{prefix}:scope"));
    let unit = unit_id(&format!("{prefix}:apply"));
    let page = page_id(&format!("{prefix}:page"));

    builder
        .register_module(SettingsModuleDefinition {
            id: module.clone(),
            label: label.to_owned(),
            order,
        })
        .unwrap();
    builder
        .register_section(SettingsSectionDefinition {
            id: section.clone(),
            module_id: module.clone(),
            label: label.to_owned(),
            order,
        })
        .unwrap();
    builder
        .register_renderer(SettingsRendererDefinition {
            id: renderer.clone(),
            module_id: module.clone(),
        })
        .unwrap();
    builder
        .register_scope(SettingsScopeDefinition {
            id: scope.clone(),
            module_id: module.clone(),
        })
        .unwrap();
    builder
        .register_apply_unit(SettingsApplyUnitDefinition {
            id: unit.clone(),
            module_id: module.clone(),
            scope_id: scope.clone(),
            timing,
            reset_supported: features.reset,
        })
        .unwrap();
    builder
        .register_page(SettingsPageDefinition {
            id: page,
            module_id: module,
            section_id: section,
            renderer_id: renderer,
            label: label.to_owned(),
            keywords: vec![label.to_ascii_lowercase(), prefix.to_owned()],
            order,
            anchors: vec![SettingsAnchorDefinition {
                id: anchor_id(&format!("{prefix}:anchor")),
                label: Some(format!("{label} details")),
                order: 0,
            }],
            required_capabilities: required_capabilities
                .iter()
                .map(|value| capability_id(value))
                .collect(),
            readable_scope_ids: vec![scope],
            writable_apply_unit_ids: vec![unit],
            features,
        })
        .unwrap();
}

pub fn register_capability(
    builder: &mut SettingsRegistryBuilder,
    owner_prefix: &str,
    capability: &str,
) {
    builder
        .register_capability(SettingsCapabilityDefinition {
            id: capability_id(capability),
            module_id: module_id(&format!("{owner_prefix}:module")),
        })
        .unwrap();
}

pub fn page_ids(registry: &SettingsRegistry) -> Vec<String> {
    registry
        .pages()
        .map(|page| page.id.as_str().to_owned())
        .collect()
}
