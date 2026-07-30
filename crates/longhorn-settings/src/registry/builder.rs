use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use longhorn_core::{
    SettingsApplyUnitId, SettingsCapabilityId, SettingsModuleId, SettingsPageId,
    SettingsRendererId, SettingsScopeId, SettingsSectionId,
};

use crate::{
    SettingsApplyUnitDefinition, SettingsCapabilityDefinition, SettingsLimits,
    SettingsModuleDefinition, SettingsPageDefinition, SettingsRegistryError,
    SettingsRegistryErrorCode, SettingsRendererDefinition, SettingsScopeDefinition,
    SettingsSectionDefinition, error::registry_error,
};

use super::{
    digest::compute_digest,
    identity::{SettingsRegistryDigest, SettingsRegistryGeneration},
    model::SettingsRegistry,
    validation::{validate_declarations, validate_limits},
};

/// Mutable pre-seal collection of settings declarations.
#[derive(Clone, Debug)]
pub struct SettingsRegistryBuilder {
    pub(super) generation: SettingsRegistryGeneration,
    pub(super) limits: SettingsLimits,
    pub(super) modules: BTreeMap<SettingsModuleId, SettingsModuleDefinition>,
    pub(super) sections: BTreeMap<SettingsSectionId, SettingsSectionDefinition>,
    pub(super) pages: BTreeMap<SettingsPageId, SettingsPageDefinition>,
    pub(super) renderers: BTreeMap<SettingsRendererId, SettingsRendererDefinition>,
    pub(super) scopes: BTreeMap<SettingsScopeId, SettingsScopeDefinition>,
    pub(super) apply_units: BTreeMap<SettingsApplyUnitId, SettingsApplyUnitDefinition>,
    pub(super) capabilities: BTreeMap<SettingsCapabilityId, SettingsCapabilityDefinition>,
}

impl SettingsRegistryBuilder {
    /// Starts one registry generation with explicit limits.
    #[must_use]
    pub fn new(generation: SettingsRegistryGeneration, limits: SettingsLimits) -> Self {
        Self {
            generation,
            limits,
            modules: BTreeMap::new(),
            sections: BTreeMap::new(),
            pages: BTreeMap::new(),
            renderers: BTreeMap::new(),
            scopes: BTreeMap::new(),
            apply_units: BTreeMap::new(),
            capabilities: BTreeMap::new(),
        }
    }

    /// Registers one module, rejecting an id already registered in this category.
    pub fn register_module(
        &mut self,
        definition: SettingsModuleDefinition,
    ) -> Result<(), SettingsRegistryError> {
        insert_unique(
            &mut self.modules,
            definition.id.clone(),
            definition,
            "module",
        )
    }

    /// Registers one section, rejecting an id already registered in this category.
    pub fn register_section(
        &mut self,
        definition: SettingsSectionDefinition,
    ) -> Result<(), SettingsRegistryError> {
        insert_unique(
            &mut self.sections,
            definition.id.clone(),
            definition,
            "section",
        )
    }

    /// Registers one page, rejecting an id already registered in this category.
    pub fn register_page(
        &mut self,
        definition: SettingsPageDefinition,
    ) -> Result<(), SettingsRegistryError> {
        insert_unique(&mut self.pages, definition.id.clone(), definition, "page")
    }

    /// Registers one renderer key.
    pub fn register_renderer(
        &mut self,
        definition: SettingsRendererDefinition,
    ) -> Result<(), SettingsRegistryError> {
        insert_unique(
            &mut self.renderers,
            definition.id.clone(),
            definition,
            "renderer",
        )
    }

    /// Registers one readable settings scope.
    pub fn register_scope(
        &mut self,
        definition: SettingsScopeDefinition,
    ) -> Result<(), SettingsRegistryError> {
        insert_unique(&mut self.scopes, definition.id.clone(), definition, "scope")
    }

    /// Registers one failure-atomic apply unit.
    pub fn register_apply_unit(
        &mut self,
        definition: SettingsApplyUnitDefinition,
    ) -> Result<(), SettingsRegistryError> {
        insert_unique(
            &mut self.apply_units,
            definition.id.clone(),
            definition,
            "apply unit",
        )
    }

    /// Registers one composition capability.
    pub fn register_capability(
        &mut self,
        definition: SettingsCapabilityDefinition,
    ) -> Result<(), SettingsRegistryError> {
        insert_unique(
            &mut self.capabilities,
            definition.id.clone(),
            definition,
            "capability",
        )
    }

    /// Validates, admits, canonicalizes, and seals this registry generation.
    pub fn seal(
        self,
        composed_capabilities: impl IntoIterator<Item = SettingsCapabilityId>,
    ) -> Result<SettingsRegistry, SettingsRegistryError> {
        validate_limits(&self)?;
        validate_declarations(&self)?;

        let composed_capabilities: BTreeSet<_> = composed_capabilities.into_iter().collect();
        for capability_id in &composed_capabilities {
            if !self.capabilities.contains_key(capability_id) {
                return Err(registry_error(
                    SettingsRegistryErrorCode::UnknownComposedCapability,
                    format!("composed capability {capability_id} is not registered"),
                ));
            }
        }

        let mut modules: Vec<_> = self.modules.into_values().collect();
        modules.sort_by(|left, right| {
            left.order
                .cmp(&right.order)
                .then_with(|| left.id.cmp(&right.id))
        });
        let module_rank: BTreeMap<_, _> = modules
            .iter()
            .enumerate()
            .map(|(rank, module)| (module.id.clone(), rank))
            .collect();

        let mut pages: Vec<_> = self
            .pages
            .into_values()
            .filter(|page| {
                page.required_capabilities
                    .iter()
                    .all(|required| composed_capabilities.contains(required))
            })
            .map(canonicalize_page)
            .collect();

        let admitted_sections: BTreeSet<_> =
            pages.iter().map(|page| page.section_id.clone()).collect();
        let mut sections: Vec<_> = self
            .sections
            .into_values()
            .filter(|section| admitted_sections.contains(&section.id))
            .collect();
        sections.sort_by(|left, right| {
            module_rank
                .get(&left.module_id)
                .cmp(&module_rank.get(&right.module_id))
                .then_with(|| left.order.cmp(&right.order))
                .then_with(|| left.id.cmp(&right.id))
        });
        let section_rank: BTreeMap<_, _> = sections
            .iter()
            .enumerate()
            .map(|(rank, section)| (section.id.clone(), rank))
            .collect();
        pages.sort_by(|left, right| {
            section_rank
                .get(&left.section_id)
                .cmp(&section_rank.get(&right.section_id))
                .then_with(|| left.order.cmp(&right.order))
                .then_with(|| left.id.cmp(&right.id))
        });

        let mut renderers: Vec<_> = self.renderers.into_values().collect();
        renderers.sort_by(|left, right| left.id.cmp(&right.id));
        let mut scopes: Vec<_> = self.scopes.into_values().collect();
        scopes.sort_by(|left, right| left.id.cmp(&right.id));
        let mut apply_units: Vec<_> = self.apply_units.into_values().collect();
        apply_units.sort_by(|left, right| left.id.cmp(&right.id));
        let mut capabilities: Vec<_> = self.capabilities.into_values().collect();
        capabilities.sort_by(|left, right| left.id.cmp(&right.id));

        let mut registry = SettingsRegistry {
            generation: self.generation,
            digest: SettingsRegistryDigest::placeholder(),
            limits: self.limits,
            composed_capabilities,
            modules,
            sections,
            pages,
            renderers,
            scopes,
            apply_units,
            capabilities,
        };
        registry.digest = compute_digest(&registry)?;
        Ok(registry)
    }
}

fn insert_unique<K, V>(
    map: &mut BTreeMap<K, V>,
    id: K,
    value: V,
    category: &str,
) -> Result<(), SettingsRegistryError>
where
    K: Ord + fmt::Display,
{
    if map.contains_key(&id) {
        return Err(registry_error(
            SettingsRegistryErrorCode::DuplicateId,
            format!("duplicate settings {category} {id}"),
        ));
    }
    map.insert(id, value);
    Ok(())
}

fn canonicalize_page(mut page: SettingsPageDefinition) -> SettingsPageDefinition {
    page.keywords.sort();
    page.keywords.dedup();
    page.anchors.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then_with(|| left.id.cmp(&right.id))
    });
    page.required_capabilities.sort();
    page.readable_scope_ids.sort();
    page.writable_apply_unit_ids.sort();
    page
}
