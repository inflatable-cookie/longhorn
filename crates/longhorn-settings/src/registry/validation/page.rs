use std::{collections::BTreeSet, fmt};

use longhorn_core::{SettingsModuleId, SettingsPageId};

use crate::{
    SettingsPageDefinition, SettingsRegistryError, SettingsRegistryErrorCode, error::registry_error,
};

use super::super::builder::SettingsRegistryBuilder;

pub(super) fn validate_page(
    builder: &SettingsRegistryBuilder,
    page: &SettingsPageDefinition,
    anchors: &mut BTreeSet<longhorn_core::SettingsAnchorId>,
) -> Result<(), SettingsRegistryError> {
    require_module(builder, &page.module_id, "page", &page.id)?;
    validate_navigation(builder, page, anchors)?;
    validate_authority(builder, page)?;
    Ok(())
}

fn validate_navigation(
    builder: &SettingsRegistryBuilder,
    page: &SettingsPageDefinition,
    anchors: &mut BTreeSet<longhorn_core::SettingsAnchorId>,
) -> Result<(), SettingsRegistryError> {
    let section = builder.sections.get(&page.section_id).ok_or_else(|| {
        registry_error(
            SettingsRegistryErrorCode::MissingReference,
            format!(
                "page {} references unknown section {}",
                page.id, page.section_id
            ),
        )
    })?;
    if section.module_id != page.module_id {
        return Err(registry_error(
            SettingsRegistryErrorCode::OwnershipMismatch,
            format!(
                "page {} and section {} have different owners",
                page.id, section.id
            ),
        ));
    }
    if !builder.renderers.contains_key(&page.renderer_id) {
        return Err(registry_error(
            SettingsRegistryErrorCode::MissingReference,
            format!(
                "page {} references unknown renderer {}",
                page.id, page.renderer_id
            ),
        ));
    }
    validate_text(
        &page.label,
        builder.limits.maximum_label_bytes,
        "page label",
    )?;
    validate_page_items(
        page.keywords.len(),
        builder.limits.maximum_keywords_per_page,
        &page.id,
        "keywords",
    )?;
    for keyword in &page.keywords {
        validate_text(
            keyword,
            builder.limits.maximum_keyword_bytes,
            "page keyword",
        )?;
    }
    validate_page_items(
        page.anchors.len(),
        builder.limits.maximum_anchors_per_page,
        &page.id,
        "anchors",
    )?;
    for anchor in &page.anchors {
        if !anchors.insert(anchor.id.clone()) {
            return Err(registry_error(
                SettingsRegistryErrorCode::DuplicateAnchor,
                format!("duplicate settings anchor {}", anchor.id),
            ));
        }
        if let Some(label) = &anchor.label {
            validate_text(label, builder.limits.maximum_label_bytes, "anchor label")?;
        }
    }
    Ok(())
}

fn validate_authority(
    builder: &SettingsRegistryBuilder,
    page: &SettingsPageDefinition,
) -> Result<(), SettingsRegistryError> {
    require_unique(
        &page.required_capabilities,
        &page.id,
        "capability reference",
    )?;
    require_unique(&page.readable_scope_ids, &page.id, "scope reference")?;
    require_unique(
        &page.writable_apply_unit_ids,
        &page.id,
        "apply-unit reference",
    )?;
    for capability_id in &page.required_capabilities {
        if !builder.capabilities.contains_key(capability_id) {
            return Err(registry_error(
                SettingsRegistryErrorCode::MissingReference,
                format!(
                    "page {} references unknown capability {capability_id}",
                    page.id
                ),
            ));
        }
    }
    for scope_id in &page.readable_scope_ids {
        if !builder.scopes.contains_key(scope_id) {
            return Err(registry_error(
                SettingsRegistryErrorCode::MissingReference,
                format!("page {} references unknown scope {scope_id}", page.id),
            ));
        }
    }
    for unit_id in &page.writable_apply_unit_ids {
        let unit = builder.apply_units.get(unit_id).ok_or_else(|| {
            registry_error(
                SettingsRegistryErrorCode::MissingReference,
                format!("page {} references unknown apply unit {unit_id}", page.id),
            )
        })?;
        if unit.module_id != page.module_id {
            return Err(registry_error(
                SettingsRegistryErrorCode::OwnershipMismatch,
                format!("page {} cannot write apply unit {unit_id}", page.id),
            ));
        }
    }
    validate_reset_support(builder, page)
}

fn validate_reset_support(
    builder: &SettingsRegistryBuilder,
    page: &SettingsPageDefinition,
) -> Result<(), SettingsRegistryError> {
    if page.features.reset
        && !page.writable_apply_unit_ids.iter().any(|id| {
            builder
                .apply_units
                .get(id)
                .is_some_and(|unit| unit.reset_supported)
        })
    {
        return Err(registry_error(
            SettingsRegistryErrorCode::MissingReference,
            format!(
                "page {} exposes reset without a reset-capable apply unit",
                page.id
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

fn validate_page_items(
    actual: usize,
    maximum: usize,
    page_id: &SettingsPageId,
    category: &str,
) -> Result<(), SettingsRegistryError> {
    if actual > maximum {
        Err(registry_error(
            SettingsRegistryErrorCode::LimitExceeded,
            format!("page {page_id} has {actual} {category}; maximum is {maximum}"),
        ))
    } else {
        Ok(())
    }
}

fn require_unique<T: Ord + fmt::Display>(
    values: &[T],
    page_id: &SettingsPageId,
    category: &str,
) -> Result<(), SettingsRegistryError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(registry_error(
                SettingsRegistryErrorCode::DuplicateId,
                format!("page {page_id} repeats {category} {value}"),
            ));
        }
    }
    Ok(())
}
