use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use longhorn_core::{LayoutContainerId, SurfaceId, WindowId};

use crate::{SurfaceDocument, SurfaceLimits};

/// Stable category for invalid current Surface state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceValidationCode {
    /// Surface count exceeded configured limits.
    TooManySurfaces,
    /// Participating-window count exceeded configured limits.
    TooManyWindows,
    /// A Surface had no declared candidate host.
    MissingHostPreference,
    /// One Surface exceeded its candidate-host limit.
    TooManyHostPreferences,
    /// A label exceeded its configured UTF-8 byte limit.
    LabelTooLong,
    /// A Surface id appeared more than once.
    DuplicateSurface,
    /// A participating-window id appeared more than once.
    DuplicateWindow,
    /// One layout container was bound to more than one Surface.
    DuplicateLayoutContainerBinding,
    /// A host preference referenced a non-participating window.
    UnknownHostWindow,
    /// A Surface repeated one candidate window.
    DuplicateHostPreference,
    /// Two Surfaces declared the same order in one candidate window.
    DuplicateHostOrder,
    /// Candidate-window order was not a complete zero-based sequence.
    IncompleteHostOrder,
    /// A participating window selected a Surface outside its declared members.
    ActiveSurfaceNotHosted,
    /// Valid state was not in canonical normalized form.
    NonCanonicalDocument,
}

/// Invalid current Surface document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceValidationError {
    code: SurfaceValidationCode,
    detail: String,
}

impl SurfaceValidationError {
    /// Returns the stable validation category.
    #[must_use]
    pub const fn code(&self) -> SurfaceValidationCode {
        self.code
    }

    /// Returns the human-readable diagnostic.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for SurfaceValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for SurfaceValidationError {}

/// Validates all current-schema Surface document invariants.
pub fn validate_document(
    limits: SurfaceLimits,
    document: &SurfaceDocument,
) -> Result<(), SurfaceValidationError> {
    if document.surfaces().len() > limits.maximum_surfaces() {
        return Err(validation_error(
            SurfaceValidationCode::TooManySurfaces,
            format!(
                "{} Surfaces exceed limit {}",
                document.surfaces().len(),
                limits.maximum_surfaces()
            ),
        ));
    }
    if document.windows().len() > limits.maximum_windows() {
        return Err(validation_error(
            SurfaceValidationCode::TooManyWindows,
            format!(
                "{} participating windows exceed limit {}",
                document.windows().len(),
                limits.maximum_windows()
            ),
        ));
    }

    let mut windows = BTreeSet::<&WindowId>::new();
    for window in document.windows() {
        if !windows.insert(window.id()) {
            return Err(validation_error(
                SurfaceValidationCode::DuplicateWindow,
                format!("duplicate participating window {}", window.id()),
            ));
        }
    }

    let mut surface_ids = BTreeSet::<&SurfaceId>::new();
    let mut container_ids = BTreeSet::<&LayoutContainerId>::new();
    let mut orders_by_window = BTreeMap::<&WindowId, BTreeSet<u32>>::new();
    let mut members_by_window = BTreeMap::<&WindowId, BTreeSet<&SurfaceId>>::new();

    for surface in document.surfaces() {
        if !surface_ids.insert(surface.id()) {
            return Err(validation_error(
                SurfaceValidationCode::DuplicateSurface,
                format!("duplicate Surface {}", surface.id()),
            ));
        }
        if !container_ids.insert(surface.layout_container_id()) {
            return Err(validation_error(
                SurfaceValidationCode::DuplicateLayoutContainerBinding,
                format!(
                    "layout container {} is bound to more than one Surface",
                    surface.layout_container_id()
                ),
            ));
        }
        if let Some(label) = surface.label()
            && label.len() > limits.maximum_label_bytes()
        {
            return Err(validation_error(
                SurfaceValidationCode::LabelTooLong,
                format!(
                    "Surface {} label is {} bytes; limit is {}",
                    surface.id(),
                    label.len(),
                    limits.maximum_label_bytes()
                ),
            ));
        }
        if surface.host_preferences().is_empty() {
            return Err(validation_error(
                SurfaceValidationCode::MissingHostPreference,
                format!("Surface {} has no candidate host", surface.id()),
            ));
        }
        if surface.host_preferences().len() > limits.maximum_host_preferences_per_surface() {
            return Err(validation_error(
                SurfaceValidationCode::TooManyHostPreferences,
                format!(
                    "Surface {} has {} host preferences; limit is {}",
                    surface.id(),
                    surface.host_preferences().len(),
                    limits.maximum_host_preferences_per_surface()
                ),
            ));
        }

        let mut preferred_windows = BTreeSet::<&WindowId>::new();
        for preference in surface.host_preferences() {
            if !windows.contains(preference.window_id()) {
                return Err(validation_error(
                    SurfaceValidationCode::UnknownHostWindow,
                    format!(
                        "Surface {} references unknown participating window {}",
                        surface.id(),
                        preference.window_id()
                    ),
                ));
            }
            if !preferred_windows.insert(preference.window_id()) {
                return Err(validation_error(
                    SurfaceValidationCode::DuplicateHostPreference,
                    format!(
                        "Surface {} repeats host window {}",
                        surface.id(),
                        preference.window_id()
                    ),
                ));
            }

            let orders = orders_by_window.entry(preference.window_id()).or_default();
            if !orders.insert(preference.order()) {
                return Err(validation_error(
                    SurfaceValidationCode::DuplicateHostOrder,
                    format!(
                        "window {} repeats Surface order {}",
                        preference.window_id(),
                        preference.order()
                    ),
                ));
            }
            members_by_window
                .entry(preference.window_id())
                .or_default()
                .insert(surface.id());
        }
    }

    for (window_id, orders) in &orders_by_window {
        let complete = (0..orders.len()).all(|index| {
            u32::try_from(index)
                .ok()
                .is_some_and(|order| orders.contains(&order))
        });
        if !complete {
            return Err(validation_error(
                SurfaceValidationCode::IncompleteHostOrder,
                format!("window {window_id} Surface order is not a complete zero-based sequence"),
            ));
        }
    }

    for window in document.windows() {
        if let Some(active_surface_id) = window.active_surface_id()
            && !members_by_window
                .get(window.id())
                .is_some_and(|members| members.contains(active_surface_id))
        {
            return Err(validation_error(
                SurfaceValidationCode::ActiveSurfaceNotHosted,
                format!(
                    "active Surface {} is not a declared member of window {}",
                    active_surface_id,
                    window.id()
                ),
            ));
        }
    }

    Ok(())
}

/// Returns one canonical structural ordering for a valid Surface document.
pub fn normalize_document(
    limits: SurfaceLimits,
    document: &SurfaceDocument,
) -> Result<SurfaceDocument, SurfaceValidationError> {
    validate_document(limits, document)?;
    let mut normalized = document.clone();
    normalized
        .surfaces_mut()
        .sort_by(|left, right| left.id().cmp(right.id()));
    normalized
        .windows_mut()
        .sort_by(|left, right| left.id().cmp(right.id()));
    validate_document(limits, &normalized)?;
    Ok(normalized)
}

/// Requires a valid Surface document to already use canonical structural order.
pub fn validate_normalized_document(
    limits: SurfaceLimits,
    document: &SurfaceDocument,
) -> Result<(), SurfaceValidationError> {
    let normalized = normalize_document(limits, document)?;
    if normalized == *document {
        Ok(())
    } else {
        Err(validation_error(
            SurfaceValidationCode::NonCanonicalDocument,
            "Surface document is valid but not canonically normalized",
        ))
    }
}

fn validation_error(
    code: SurfaceValidationCode,
    detail: impl Into<String>,
) -> SurfaceValidationError {
    SurfaceValidationError {
        code,
        detail: detail.into(),
    }
}
