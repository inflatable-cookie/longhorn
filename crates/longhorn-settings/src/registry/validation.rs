mod page;

use std::{collections::BTreeSet, fmt};

use longhorn_core::SettingsModuleId;

use crate::{SettingsRegistryError, SettingsRegistryErrorCode, error::registry_error};

use super::builder::SettingsRegistryBuilder;
use page::validate_page;

pub(super) fn validate_limits(
    builder: &SettingsRegistryBuilder,
) -> Result<(), SettingsRegistryError> {
    let limits = builder.limits;
    if !limits.is_valid() {
        return Err(registry_error(
            SettingsRegistryErrorCode::InvalidLimits,
            "settings limits must be nonzero and below defensive ceilings",
        ));
    }
    for (category, actual, maximum) in [
        ("modules", builder.modules.len(), limits.maximum_modules),
        ("sections", builder.sections.len(), limits.maximum_sections),
        ("pages", builder.pages.len(), limits.maximum_pages),
        (
            "renderers",
            builder.renderers.len(),
            limits.maximum_renderers,
        ),
        ("scopes", builder.scopes.len(), limits.maximum_scopes),
        (
            "apply units",
            builder.apply_units.len(),
            limits.maximum_apply_units,
        ),
        (
            "capabilities",
            builder.capabilities.len(),
            limits.maximum_capabilities,
        ),
    ] {
        if actual > maximum {
            return Err(registry_error(
                SettingsRegistryErrorCode::LimitExceeded,
                format!("{actual} settings {category} exceed limit {maximum}"),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_declarations(
    builder: &SettingsRegistryBuilder,
) -> Result<(), SettingsRegistryError> {
    for module in builder.modules.values() {
        validate_text(
            &module.label,
            builder.limits.maximum_label_bytes,
            "module label",
        )?;
    }
    for section in builder.sections.values() {
        require_module(builder, &section.module_id, "section", &section.id)?;
        validate_text(
            &section.label,
            builder.limits.maximum_label_bytes,
            "section label",
        )?;
    }
    for renderer in builder.renderers.values() {
        require_module(builder, &renderer.module_id, "renderer", &renderer.id)?;
    }
    for scope in builder.scopes.values() {
        require_module(builder, &scope.module_id, "scope", &scope.id)?;
    }
    for capability in builder.capabilities.values() {
        require_module(builder, &capability.module_id, "capability", &capability.id)?;
    }
    for unit in builder.apply_units.values() {
        validate_apply_unit(builder, unit)?;
    }

    let mut anchors = BTreeSet::new();
    for page in builder.pages.values() {
        validate_page(builder, page, &mut anchors)?;
    }
    Ok(())
}

fn validate_apply_unit(
    builder: &SettingsRegistryBuilder,
    unit: &crate::SettingsApplyUnitDefinition,
) -> Result<(), SettingsRegistryError> {
    require_module(builder, &unit.module_id, "apply unit", &unit.id)?;
    let scope = builder.scopes.get(&unit.scope_id).ok_or_else(|| {
        registry_error(
            SettingsRegistryErrorCode::MissingReference,
            format!(
                "apply unit {} references unknown scope {}",
                unit.id, unit.scope_id
            ),
        )
    })?;
    if scope.module_id != unit.module_id {
        return Err(registry_error(
            SettingsRegistryErrorCode::OwnershipMismatch,
            format!(
                "apply unit {} and scope {} have different owners",
                unit.id, scope.id
            ),
        ));
    }
    Ok(())
}

fn require_module<I: fmt::Display>(
    builder: &SettingsRegistryBuilder,
    module_id: &SettingsModuleId,
    category: &str,
    id: &I,
) -> Result<(), SettingsRegistryError> {
    if builder.modules.contains_key(module_id) {
        Ok(())
    } else {
        Err(registry_error(
            SettingsRegistryErrorCode::MissingReference,
            format!("{category} {id} references unknown module {module_id}"),
        ))
    }
}

fn validate_text(value: &str, maximum: usize, category: &str) -> Result<(), SettingsRegistryError> {
    if value.trim().is_empty() {
        return Err(registry_error(
            SettingsRegistryErrorCode::EmptyText,
            format!("{category} cannot be empty"),
        ));
    }
    if value.len() > maximum {
        return Err(registry_error(
            SettingsRegistryErrorCode::TextTooLong,
            format!("{category} is {} bytes; maximum is {maximum}", value.len()),
        ));
    }
    Ok(())
}
