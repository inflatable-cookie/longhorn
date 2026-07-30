use std::time::Duration;

use longhorn_config::{DurabilityRequirement, MutationOptions};
use longhorn_core::{
    SettingsApplyUnitId, SettingsEntryId, SettingsModuleId, SettingsPageId, SettingsRendererId,
    SettingsRequestId, SettingsScopeId, SettingsSectionId,
};
use longhorn_settings::{
    SettingsApplyCommand, SettingsApplyUnitDefinition, SettingsLimits, SettingsLoadCommand,
    SettingsModuleDefinition, SettingsMutationTiming, SettingsOpaqueValue, SettingsPageDefinition,
    SettingsPageFeatures, SettingsRegistry, SettingsRegistryBuilder, SettingsRegistryGeneration,
    SettingsRendererDefinition, SettingsResetCommand, SettingsScopeDefinition,
    SettingsSectionDefinition,
};

pub(crate) fn sealed_registry(timing: SettingsMutationTiming) -> SettingsRegistry {
    let module = SettingsModuleId::new("preferences:module").unwrap();
    let section = SettingsSectionId::new("preferences:section").unwrap();
    let renderer = SettingsRendererId::new("preferences:renderer").unwrap();
    let scope = scope_id();
    let unit = unit_id();
    let page = page_id();
    let mut builder = SettingsRegistryBuilder::new(
        SettingsRegistryGeneration::new(7),
        SettingsLimits::default(),
    );
    builder
        .register_module(SettingsModuleDefinition {
            id: module.clone(),
            label: "Preferences".into(),
            order: 0,
        })
        .unwrap();
    builder
        .register_section(SettingsSectionDefinition {
            id: section.clone(),
            module_id: module.clone(),
            label: "Preferences".into(),
            order: 0,
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
            reset_supported: true,
        })
        .unwrap();
    builder
        .register_page(SettingsPageDefinition {
            id: page,
            module_id: module,
            section_id: section,
            renderer_id: renderer,
            label: "Preferences".into(),
            keywords: vec!["preferences".into()],
            order: 0,
            anchors: vec![],
            required_capabilities: vec![],
            readable_scope_ids: vec![scope],
            writable_apply_unit_ids: vec![unit],
            features: SettingsPageFeatures {
                reset: true,
                ..SettingsPageFeatures::default()
            },
        })
        .unwrap();
    builder.seal([]).unwrap()
}

pub(crate) fn load_command() -> SettingsLoadCommand {
    SettingsLoadCommand {
        protocol_version: longhorn_settings::SettingsProtocolVersion::CURRENT,
        request_id: SettingsRequestId::new("request:load").unwrap(),
        registry_generation: SettingsRegistryGeneration::new(7),
        scope_id: scope_id(),
        known_authority: None,
    }
}

pub(crate) fn apply_command(
    authority: longhorn_settings::SettingsAuthorityExpectation,
    intent: SettingsOpaqueValue,
) -> SettingsApplyCommand {
    SettingsApplyCommand {
        protocol_version: longhorn_settings::SettingsProtocolVersion::CURRENT,
        request_id: SettingsRequestId::new("request:apply").unwrap(),
        page_id: page_id(),
        apply_unit_id: unit_id(),
        scope_id: scope_id(),
        authority,
        intent,
    }
}

pub(crate) fn reset_command(
    authority: longhorn_settings::SettingsAuthorityExpectation,
    entry_ids: Vec<SettingsEntryId>,
) -> SettingsResetCommand {
    SettingsResetCommand {
        protocol_version: longhorn_settings::SettingsProtocolVersion::CURRENT,
        request_id: SettingsRequestId::new("request:reset").unwrap(),
        page_id: page_id(),
        apply_unit_id: unit_id(),
        scope_id: scope_id(),
        authority,
        entry_ids,
    }
}

pub(crate) fn options() -> MutationOptions {
    MutationOptions::new(Duration::from_secs(2), DurabilityRequirement::Durable)
}

pub(crate) fn page_id() -> SettingsPageId {
    SettingsPageId::new("preferences:page").unwrap()
}

pub(crate) fn unit_id() -> SettingsApplyUnitId {
    SettingsApplyUnitId::new("preferences:apply").unwrap()
}

pub(crate) fn scope_id() -> SettingsScopeId {
    SettingsScopeId::new("preferences:scope").unwrap()
}

pub(crate) fn entry_id(value: &str) -> SettingsEntryId {
    SettingsEntryId::new(value).unwrap()
}
