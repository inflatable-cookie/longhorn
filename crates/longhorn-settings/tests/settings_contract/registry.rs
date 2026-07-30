mod boundary;
mod rejection;
mod seal;

use longhorn_core::{
    SettingsApplyUnitId, SettingsModuleId, SettingsPageId, SettingsRendererId, SettingsScopeId,
    SettingsSectionId,
};
use longhorn_settings::{
    SettingsLimits, SettingsMutationTiming, SettingsPageDefinition, SettingsPageFeatures,
    SettingsRegistryBuilder, SettingsRegistryGeneration,
};

use super::support::register_module_page;

pub(super) fn page_with(
    module_id: SettingsModuleId,
    section_id: SettingsSectionId,
    renderer_id: SettingsRendererId,
    readable_scope_ids: Vec<SettingsScopeId>,
    writable_apply_unit_ids: Vec<SettingsApplyUnitId>,
) -> SettingsPageDefinition {
    SettingsPageDefinition {
        id: SettingsPageId::new("second:page").unwrap(),
        module_id,
        section_id,
        renderer_id,
        label: "Second".into(),
        keywords: vec![],
        order: 10,
        anchors: vec![],
        required_capabilities: vec![],
        readable_scope_ids,
        writable_apply_unit_ids,
        features: SettingsPageFeatures::default(),
    }
}

pub(super) fn two_module_registry(
    reverse: bool,
    generation: u64,
) -> longhorn_settings::SettingsRegistry {
    let mut builder = SettingsRegistryBuilder::new(
        SettingsRegistryGeneration::new(generation),
        SettingsLimits::default(),
    );
    let definitions = [
        ("alpha", "Alpha", SettingsMutationTiming::Immediate),
        ("zeta", "Zeta", SettingsMutationTiming::Staged),
    ];
    if reverse {
        for (prefix, label, timing) in definitions.into_iter().rev() {
            register_module_page(
                &mut builder,
                prefix,
                label,
                0,
                timing,
                &[],
                SettingsPageFeatures::default(),
            );
        }
    } else {
        for (prefix, label, timing) in definitions {
            register_module_page(
                &mut builder,
                prefix,
                label,
                0,
                timing,
                &[],
                SettingsPageFeatures::default(),
            );
        }
    }
    builder.seal([]).unwrap()
}
