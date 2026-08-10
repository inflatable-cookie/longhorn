use std::collections::BTreeMap;

use longhorn_core::{LayoutSchemaId, PanelDefinitionId, RegionId};

use super::{
    DefinitionErrorCode, DefinitionLookupError, DefinitionRegistryError, LayoutLimits,
    LayoutSchemaDefinition, PanelDefinition, definition_error, validate_panel, validate_schema,
};

/// Validated immutable layout and panel definition registry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutDefinitionRegistry {
    limits: LayoutLimits,
    schemas: BTreeMap<LayoutSchemaId, LayoutSchemaDefinition>,
    panel_definitions: BTreeMap<PanelDefinitionId, PanelDefinition>,
}

impl LayoutDefinitionRegistry {
    /// Validates and constructs a registry.
    pub fn new(
        limits: LayoutLimits,
        schemas: impl IntoIterator<Item = LayoutSchemaDefinition>,
        panel_definitions: impl IntoIterator<Item = PanelDefinition>,
    ) -> Result<Self, DefinitionRegistryError> {
        let schemas: Vec<_> = schemas.into_iter().collect();
        let panel_definitions: Vec<_> = panel_definitions.into_iter().collect();

        if schemas.is_empty() {
            return Err(definition_error(
                DefinitionErrorCode::EmptyRegistry,
                "at least one layout schema is required",
            ));
        }
        if schemas.len() > limits.maximum_schemas() {
            return Err(definition_error(
                DefinitionErrorCode::TooManySchemas,
                format!(
                    "{} schemas exceed limit {}",
                    schemas.len(),
                    limits.maximum_schemas()
                ),
            ));
        }
        if panel_definitions.len() > limits.maximum_panel_definitions() {
            return Err(definition_error(
                DefinitionErrorCode::TooManyPanelDefinitions,
                format!(
                    "{} panel definitions exceed limit {}",
                    panel_definitions.len(),
                    limits.maximum_panel_definitions()
                ),
            ));
        }

        let mut schema_map = BTreeMap::new();
        for mut schema in schemas {
            validate_schema(&schema, limits)?;
            schema.regions.sort_by(|left, right| {
                left.order
                    .cmp(&right.order)
                    .then_with(|| left.id.cmp(&right.id))
            });
            schema.sizing_slots.sort_by(|left, right| {
                left.order
                    .cmp(&right.order)
                    .then_with(|| left.id.cmp(&right.id))
            });
            let id = schema.id.clone();
            if schema_map.insert(id.clone(), schema).is_some() {
                return Err(definition_error(
                    DefinitionErrorCode::DuplicateSchema,
                    format!("duplicate layout schema {id}"),
                ));
            }
        }

        let mut panel_map = BTreeMap::new();
        for panel in panel_definitions {
            validate_panel(&panel, schema_map.values())?;
            let id = panel.id.clone();
            if panel_map.insert(id.clone(), panel).is_some() {
                return Err(definition_error(
                    DefinitionErrorCode::DuplicatePanelDefinition,
                    format!("duplicate panel definition {id}"),
                ));
            }
        }

        Ok(Self {
            limits,
            schemas: schema_map,
            panel_definitions: panel_map,
        })
    }

    /// Returns explicit registry and document limits.
    #[must_use]
    pub const fn limits(&self) -> LayoutLimits {
        self.limits
    }

    /// Returns schemas in stable id order.
    pub fn schemas(&self) -> impl ExactSizeIterator<Item = &LayoutSchemaDefinition> {
        self.schemas.values()
    }

    /// Returns panel definitions in stable id order.
    pub fn panel_definitions(&self) -> impl ExactSizeIterator<Item = &PanelDefinition> {
        self.panel_definitions.values()
    }

    /// Returns one registered schema.
    #[must_use]
    pub fn schema(&self, id: &LayoutSchemaId) -> Option<&LayoutSchemaDefinition> {
        self.schemas.get(id)
    }

    /// Returns one registered panel definition.
    #[must_use]
    pub fn panel_definition(&self, id: &PanelDefinitionId) -> Option<&PanelDefinition> {
        self.panel_definitions.get(id)
    }

    /// Returns whether a panel definition allows one region in a schema.
    pub fn is_panel_allowed_in(
        &self,
        schema_id: &LayoutSchemaId,
        panel_definition_id: &PanelDefinitionId,
        region_id: &RegionId,
    ) -> Result<bool, DefinitionLookupError> {
        let schema = self
            .schema(schema_id)
            .ok_or_else(|| DefinitionLookupError::UnknownSchema(schema_id.clone()))?;
        let panel = self.panel_definition(panel_definition_id).ok_or_else(|| {
            DefinitionLookupError::UnknownPanelDefinition(panel_definition_id.clone())
        })?;
        let region = schema
            .region(region_id)
            .ok_or_else(|| DefinitionLookupError::UnknownRegion(region_id.clone()))?;

        Ok(panel
            .allowed_placements
            .iter()
            .any(|selector| selector.matches(region)))
    }

    /// Resolves allowed regions in schema order.
    pub fn eligible_regions(
        &self,
        schema_id: &LayoutSchemaId,
        panel_definition_id: &PanelDefinitionId,
    ) -> Result<Vec<RegionId>, DefinitionLookupError> {
        let schema = self
            .schema(schema_id)
            .ok_or_else(|| DefinitionLookupError::UnknownSchema(schema_id.clone()))?;
        let panel = self.panel_definition(panel_definition_id).ok_or_else(|| {
            DefinitionLookupError::UnknownPanelDefinition(panel_definition_id.clone())
        })?;

        Ok(schema
            .regions()
            .iter()
            .filter(|region| {
                panel
                    .allowed_placements
                    .iter()
                    .any(|selector| selector.matches(region))
            })
            .map(|region| region.id().clone())
            .collect())
    }

    /// Resolves the first compatible default region in selector and schema order.
    pub fn default_region(
        &self,
        schema_id: &LayoutSchemaId,
        panel_definition_id: &PanelDefinitionId,
    ) -> Result<Option<RegionId>, DefinitionLookupError> {
        let schema = self
            .schema(schema_id)
            .ok_or_else(|| DefinitionLookupError::UnknownSchema(schema_id.clone()))?;
        let panel = self.panel_definition(panel_definition_id).ok_or_else(|| {
            DefinitionLookupError::UnknownPanelDefinition(panel_definition_id.clone())
        })?;

        for selector in panel.default_placements() {
            if let Some(region) = schema.regions().iter().find(|region| {
                selector.matches(region)
                    && panel
                        .allowed_placements()
                        .iter()
                        .any(|allowed| allowed.matches(region))
            }) {
                return Ok(Some(region.id().clone()));
            }
        }
        Ok(None)
    }
}
