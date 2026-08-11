use longhorn_core::SurfaceId;

use crate::{LayoutDefinitionRegistry, SurfaceDocument, SurfaceHostPreference, SurfaceRecord};

use super::{
    OperationRejection, SurfaceMutationOutcome, SurfaceMutationRejectionCode, materialize_schema,
    operation_rejection, require_fresh_surface,
};

pub(super) fn duplicate_surface(
    document: &mut SurfaceDocument,
    registry: &LayoutDefinitionRegistry,
    source_surface_id: &SurfaceId,
    surface_id: &SurfaceId,
) -> Result<SurfaceMutationOutcome, OperationRejection> {
    require_fresh_surface(document, surface_id)?;
    let source = document
        .surface(source_surface_id)
        .ok_or_else(|| {
            operation_rejection(
                SurfaceMutationRejectionCode::UnknownSurface,
                format!("unknown source Surface {source_surface_id}"),
            )
        })?
        .clone();

    let mut duplicate_preferences = Vec::with_capacity(source.host_preferences().len());
    for source_preference in source.host_preferences() {
        let insertion_order = source_preference.order().checked_add(1).ok_or_else(|| {
            operation_rejection(
                SurfaceMutationRejectionCode::InvalidCandidate,
                "duplicate host order overflow",
            )
        })?;
        for existing in document.surfaces_mut() {
            if let Some(preference) = existing
                .host_preferences_mut()
                .iter_mut()
                .find(|preference| preference.window_id() == source_preference.window_id())
                && preference.order() >= insertion_order
            {
                preference.set_order(preference.order().checked_add(1).ok_or_else(|| {
                    operation_rejection(
                        SurfaceMutationRejectionCode::InvalidCandidate,
                        "shifted host order overflow",
                    )
                })?);
            }
        }
        duplicate_preferences.push(SurfaceHostPreference::new(
            source_preference.window_id().clone(),
            insertion_order,
        ));
    }

    // Duplication copies generic metadata and hosting policy, never layout
    // contents: the copy instantiates the same schema fresh, as it did when the
    // duplicate bound a new empty surface.
    let (regions, sizing_slots) = materialize_schema(registry, source.schema_id())?;
    document.surfaces_mut().push(SurfaceRecord::new(
        surface_id.clone(),
        source.schema_id().clone(),
        source.label().map(str::to_owned),
        regions,
        sizing_slots,
        duplicate_preferences,
    ));
    Ok(SurfaceMutationOutcome::SurfaceDuplicated {
        source_surface_id: source_surface_id.clone(),
        surface_id: surface_id.clone(),
    })
}
