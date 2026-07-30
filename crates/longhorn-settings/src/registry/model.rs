use std::collections::BTreeSet;

use longhorn_core::{SettingsApplyUnitId, SettingsCapabilityId, SettingsPageId};

use crate::{
    SettingsApplyUnitDefinition, SettingsCapabilityDefinition, SettingsLimits,
    SettingsModuleDefinition, SettingsPageDefinition, SettingsRendererDefinition,
    SettingsScopeDefinition, SettingsSectionDefinition,
};

use super::identity::{SettingsRegistryDigest, SettingsRegistryGeneration};

/// Validated immutable settings declarations for one host generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsRegistry {
    pub(super) generation: SettingsRegistryGeneration,
    pub(super) digest: SettingsRegistryDigest,
    pub(super) limits: SettingsLimits,
    pub(super) composed_capabilities: BTreeSet<SettingsCapabilityId>,
    pub(super) modules: Vec<SettingsModuleDefinition>,
    pub(super) sections: Vec<SettingsSectionDefinition>,
    pub(super) pages: Vec<SettingsPageDefinition>,
    pub(super) renderers: Vec<SettingsRendererDefinition>,
    pub(super) scopes: Vec<SettingsScopeDefinition>,
    pub(super) apply_units: Vec<SettingsApplyUnitDefinition>,
    pub(super) capabilities: Vec<SettingsCapabilityDefinition>,
}

impl SettingsRegistry {
    /// Returns the monotonic host generation.
    #[must_use]
    pub const fn generation(&self) -> SettingsRegistryGeneration {
        self.generation
    }

    /// Returns the digest of canonical admitted content.
    #[must_use]
    pub fn digest(&self) -> &SettingsRegistryDigest {
        &self.digest
    }

    /// Returns the explicit limits bound into this registry.
    #[must_use]
    pub const fn limits(&self) -> SettingsLimits {
        self.limits
    }

    /// Returns composed capabilities in stable id order.
    pub fn composed_capabilities(&self) -> impl ExactSizeIterator<Item = &SettingsCapabilityId> {
        self.composed_capabilities.iter()
    }

    /// Returns modules in explicit order, then stable id order.
    pub fn modules(&self) -> impl ExactSizeIterator<Item = &SettingsModuleDefinition> {
        self.modules.iter()
    }

    /// Returns admitted nonempty sections in deterministic order.
    pub fn sections(&self) -> impl ExactSizeIterator<Item = &SettingsSectionDefinition> {
        self.sections.iter()
    }

    /// Returns admitted pages in deterministic section and page order.
    pub fn pages(&self) -> impl ExactSizeIterator<Item = &SettingsPageDefinition> {
        self.pages.iter()
    }

    /// Returns registered renderer keys in stable id order.
    pub fn renderers(&self) -> impl ExactSizeIterator<Item = &SettingsRendererDefinition> {
        self.renderers.iter()
    }

    /// Returns registered scopes in stable id order.
    pub fn scopes(&self) -> impl ExactSizeIterator<Item = &SettingsScopeDefinition> {
        self.scopes.iter()
    }

    /// Returns registered apply units in stable id order.
    pub fn apply_units(&self) -> impl ExactSizeIterator<Item = &SettingsApplyUnitDefinition> {
        self.apply_units.iter()
    }

    /// Returns registered capabilities in stable id order.
    pub fn capabilities(&self) -> impl ExactSizeIterator<Item = &SettingsCapabilityDefinition> {
        self.capabilities.iter()
    }

    /// Returns one admitted page.
    #[must_use]
    pub fn page(&self, id: &SettingsPageId) -> Option<&SettingsPageDefinition> {
        self.pages.iter().find(|page| &page.id == id)
    }

    /// Returns one registered apply unit.
    #[must_use]
    pub fn apply_unit(&self, id: &SettingsApplyUnitId) -> Option<&SettingsApplyUnitDefinition> {
        self.apply_units.iter().find(|unit| &unit.id == id)
    }
}
