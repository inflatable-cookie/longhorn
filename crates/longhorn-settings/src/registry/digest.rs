use serde::Serialize;

use crate::{
    SettingsApplyUnitDefinition, SettingsCapabilityDefinition, SettingsLimits,
    SettingsModuleDefinition, SettingsPageDefinition, SettingsRegistryError,
    SettingsRegistryErrorCode, SettingsRendererDefinition, SettingsScopeDefinition,
    SettingsSectionDefinition, error::registry_error,
};

use super::{identity::SettingsRegistryDigest, model::SettingsRegistry};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistryDigestMaterial<'registry> {
    limits: SettingsLimits,
    composed_capabilities:
        &'registry std::collections::BTreeSet<longhorn_core::SettingsCapabilityId>,
    modules: &'registry [SettingsModuleDefinition],
    sections: &'registry [SettingsSectionDefinition],
    pages: &'registry [SettingsPageDefinition],
    renderers: &'registry [SettingsRendererDefinition],
    scopes: &'registry [SettingsScopeDefinition],
    apply_units: &'registry [SettingsApplyUnitDefinition],
    capabilities: &'registry [SettingsCapabilityDefinition],
}

pub(super) fn compute_digest(
    registry: &SettingsRegistry,
) -> Result<SettingsRegistryDigest, SettingsRegistryError> {
    let material = RegistryDigestMaterial {
        limits: registry.limits,
        composed_capabilities: &registry.composed_capabilities,
        modules: &registry.modules,
        sections: &registry.sections,
        pages: &registry.pages,
        renderers: &registry.renderers,
        scopes: &registry.scopes,
        apply_units: &registry.apply_units,
        capabilities: &registry.capabilities,
    };
    serde_json::to_vec(&material)
        .map(|bytes| SettingsRegistryDigest::from_bytes(&bytes))
        .map_err(|error| {
            registry_error(
                SettingsRegistryErrorCode::DigestEncoding,
                format!("could not encode settings registry digest material: {error}"),
            )
        })
}
